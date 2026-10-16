//! Blocking input executor. Command receivers never await a driver, and stop
//! or cancellation signals reach the pending admission during model/device open.
use super::{
    admission::{Admission, StartupObservation},
    owner::{Driver, Owner, Preferences, State},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub trait Speech {
    type Job: Send + 'static;
    fn cancellation(&self) -> Arc<dyn Fn() + Send + Sync>;
    fn accept(&mut self, pcm: &[f32]) -> Result<(), &'static str>;
    fn stop(self) -> Result<Self::Job, &'static str>;
}
// Jobs and PCM stay on local Rust channels, never in a renderer event.
pub use govorun_core::features::dictation::input_events::Audio as Event;
struct Signal {
    id: u64,
    admission: Arc<Admission>,
    cancel_speech: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl Signal {
    fn cancel(&self) {
        self.admission.cancel();
        if let Some(cancel) = &self.cancel_speech {
            cancel();
        }
    }
}
enum Command<S> {
    Begin {
        id: u64,
        admission: Arc<Admission>,
        preferences: Preferences,
        speech: S,
    },
    Configure(Preferences, bool, mpsc::Sender<Result<(), &'static str>>),
    Cancel(u64),
    Shutdown,
}
type Observer = Arc<dyn Fn(StartupObservation) + Send + Sync>;
type ObserverSlot = Arc<Mutex<Option<Observer>>>;
pub struct Executor<S: Send + 'static> {
    send: mpsc::Sender<Command<S>>,
    signal: Arc<Mutex<Option<Signal>>>,
    stopping: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    startup: ObserverSlot,
}
impl<S: Send + 'static> Executor<S> {
    pub fn spawn<D, R>(
        clock: Instant,
        driver: impl FnOnce() -> D + Send + 'static,
        speech: impl FnMut(&S) -> Result<R, &'static str> + Send + 'static,
        events: Arc<dyn Fn(Event<R::Job>) + Send + Sync>,
    ) -> Result<Self, &'static str>
    where
        D: Driver + 'static,
        R: Speech + 'static,
    {
        let (send, receive) = mpsc::channel();
        let signal = Arc::new(Mutex::new(None));
        let stopping = Arc::new(AtomicBool::new(false));
        let startup = ObserverSlot::default();
        let startup_observer = startup.clone();
        let slot = signal.clone();
        let stopped = stopping.clone();
        let thread = thread::Builder::new()
            .name("Govorun capture".into())
            .spawn(move || {
                let failed_signal = slot.clone();
                let failed_events = events.clone();
                let failed_stopping = stopped.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Worker {
                        owner: Owner::new(driver()),
                        speech,
                        current: None,
                        signal: slot,
                        stopping: stopped,
                        events,
                        clock,
                        startup: startup_observer,
                    }
                    .run(receive)
                }));
                if result.is_err() {
                    failed_stopping.store(true, Ordering::Release);
                    let failed = failed_signal
                        .lock()
                        .ok()
                        .and_then(|mut signal| signal.take());
                    if let Some(signal) = failed {
                        signal.cancel();
                        (failed_events)(Event::Failed {
                            id: signal.id,
                            code: "microphone_executor_unavailable",
                        });
                    }
                }
            })
            .map_err(|_| "microphone_executor_unavailable")?;
        Ok(Self {
            send,
            signal,
            stopping,
            thread: Mutex::new(Some(thread)),
            startup,
        })
    }

    pub fn observe_startup(&self, observer: Observer) -> Result<(), &'static str> {
        *self
            .startup
            .lock()
            .map_err(|_| "microphone_executor_unavailable")? = Some(observer);
        Ok(())
    }

    pub fn begin(
        &self,
        id: u64,
        admission: Arc<Admission>,
        preferences: Preferences,
        speech: S,
    ) -> Result<(), &'static str> {
        let mut slot = self
            .signal
            .lock()
            .map_err(|_| "microphone_executor_unavailable")?;
        if self.stopping.load(Ordering::Acquire) || admission.is_cancelled() {
            return Err("recording_cancelled");
        }
        if slot.is_some() {
            return Err("recording_already_active");
        }
        *slot = Some(Signal {
            id,
            admission: admission.clone(),
            cancel_speech: None,
        });
        if self
            .send
            .send(Command::Begin {
                id,
                admission,
                preferences,
                speech,
            })
            .is_err()
        {
            if let Some(signal) = slot.take() {
                signal.cancel();
            }
            return Err("microphone_executor_unavailable");
        }
        Ok(())
    }

    /// Already frozen by Core; writing this signal never waits for input open.
    pub fn release(&self, id: u64, deadline: u64) -> Result<bool, &'static str> {
        let slot = self
            .signal
            .lock()
            .map_err(|_| "microphone_executor_unavailable")?;
        if let Some(signal) = slot.as_ref().filter(|signal| signal.id == id) {
            signal.admission.release(deadline)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn configure(
        &self,
        preferences: Preferences,
        retain: bool,
    ) -> Result<mpsc::Receiver<Result<(), &'static str>>, &'static str> {
        if self.stopping.load(Ordering::Acquire) {
            return Err("recording_cancelled");
        }
        let (send, receive) = mpsc::channel();
        self.send
            .send(Command::Configure(preferences, retain, send))
            .map_err(|_| "microphone_executor_unavailable")?;
        Ok(receive)
    }

    pub fn cancel(&self) {
        if let Ok(mut slot) = self.signal.lock() {
            if let Some(signal) = slot.take() {
                signal.cancel();
                // Keep cancellation before a replacement Begin on this queue.
                let _ = self.send.send(Command::Cancel(signal.id));
            }
        }
    }

    /// Call on a blocking executor during host shutdown.
    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::Release);
        self.cancel();
        let _ = self.send.send(Command::Shutdown);
        if let Ok(mut thread) = self.thread.lock() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}
impl<S: Send + 'static> Drop for Executor<S> {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        self.cancel();
        let _ = self.send.send(Command::Shutdown);
        // Dropping on the main thread must not block on model initialization.
        // Worker observes stopping before any device or result is admitted.
    }
}

struct Worker<D: Driver, R: Speech, F> {
    owner: Owner<D>,
    speech: F,
    current: Option<(u64, R, bool)>,
    signal: Arc<Mutex<Option<Signal>>>,
    stopping: Arc<AtomicBool>,
    events: Arc<dyn Fn(Event<R::Job>) + Send + Sync>,
    clock: Instant,
    startup: ObserverSlot,
}
impl<D: Driver, R: Speech, F> Worker<D, R, F> {
    fn clear(&self, id: u64) {
        if let Ok(mut slot) = self.signal.lock() {
            if slot.as_ref().is_some_and(|signal| signal.id == id) {
                *slot = None;
            }
        }
    }
    fn failed(&mut self, id: u64, code: &'static str) {
        self.owner.cancel(id);
        if let Some((_, speech, _)) = self.current.take() {
            speech.cancellation()();
        }
        self.clear(id);
        (self.events)(Event::Failed { id, code });
    }
    fn tick(&mut self) {
        let now = self.clock.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let Some((id, speech, announced)) = self.current.as_mut() else {
            let _ = self.owner.poll(now); // Idle disconnection closes only the input.
            return;
        };
        let id = *id;
        let packet = match self.owner.poll(now) {
            Ok(packet) => packet,
            Err(code) => {
                self.failed(id, code);
                return;
            }
        };
        if packet.state == State::Cancelled {
            speech.cancellation()();
            self.current = None;
            self.clear(id);
            (self.events)(Event::Cancelled { id });
            return;
        }
        if !packet.pcm.is_empty() {
            // Cancellation may arrive after poll; never feed an already
            // revoked job while its Cancel command waits on this worker.
            let current = self.signal.lock().ok().is_some_and(|slot| {
                slot.as_ref()
                    .is_some_and(|signal| signal.id == id && !signal.admission.is_cancelled())
            });
            if !current {
                return;
            }
            if let Err(code) = speech.accept(&packet.pcm) {
                self.failed(id, code);
                return;
            }
            // Readiness is current audio, not completion of a delayed open.
            // An early stop proceeds directly to recognition without a start
            // cue/mute/readiness announcement after the user already released.
            let starting = self.signal.lock().ok().is_some_and(|slot| {
                slot.as_ref().is_some_and(|signal| {
                    signal.id == id
                        && !signal.admission.is_cancelled()
                        && signal.admission.deadline().is_none()
                })
            });
            if !*announced && starting {
                *announced = true;
                (self.events)(Event::Ready { id });
                let facts = self.signal.lock().ok().and_then(|slot| {
                    slot.as_ref()
                        .filter(|signal| signal.id == id)
                        .and_then(|signal| signal.admission.startup_observation())
                });
                let observer = self.startup.lock().ok().and_then(|slot| slot.clone());
                if let (Some(facts), Some(observer)) = (facts, observer) {
                    observer(facts);
                }
            }
        }
        if packet.state == State::Complete {
            let (_, speech, _) = self.current.take().unwrap();
            let job = speech.stop();
            self.clear(id);
            match job {
                Ok(job) => (self.events)(Event::Stopped { id, job }),
                Err(code) => (self.events)(Event::Failed { id, code }),
            }
        }
    }
    fn run<S: Send + 'static>(&mut self, receive: mpsc::Receiver<Command<S>>)
    where
        F: FnMut(&S) -> Result<R, &'static str>,
    {
        loop {
            if self.stopping.load(Ordering::Acquire) {
                break;
            }
            let command = if self.current.is_some() || self.owner.preferences().warm {
                match receive.recv_timeout(Duration::from_millis(10)) {
                    Ok(command) => Some(command),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => break,
                }
            } else {
                match receive.recv() {
                    Ok(command) => Some(command),
                    Err(_) => break,
                }
            };
            match command {
                Some(Command::Begin {
                    id,
                    admission,
                    preferences,
                    speech,
                }) => {
                    if admission.is_cancelled() {
                        self.clear(id);
                        (self.events)(Event::Cancelled { id });
                        continue;
                    }
                    let record = match (self.speech)(&speech) {
                        Ok(record) => record,
                        Err(code) => {
                            self.clear(id);
                            (self.events)(Event::Failed { id, code });
                            continue;
                        }
                    };
                    let cancel = record.cancellation();
                    if let Ok(mut slot) = self.signal.lock() {
                        if let Some(signal) = slot.as_mut().filter(|signal| signal.id == id) {
                            signal.cancel_speech = Some(cancel.clone());
                        }
                    }
                    if admission.is_cancelled() || self.stopping.load(Ordering::Acquire) {
                        cancel();
                        self.clear(id);
                        (self.events)(Event::Cancelled { id });
                        continue;
                    }
                    admission.input_opening();
                    if let Err(code) = self.owner.begin(id, admission.clone(), preferences) {
                        cancel();
                        self.clear(id);
                        (self.events)(if admission.is_cancelled() {
                            Event::Cancelled { id }
                        } else if let Some(native) = admission.native_failure() {
                            Event::FailedNative { id, code, native }
                        } else {
                            Event::Failed { id, code }
                        });
                        continue;
                    }
                    admission.input_opened();
                    self.current = Some((id, record, false));
                }
                Some(Command::Configure(preferences, retain, reply)) => {
                    let _ = reply.send(self.owner.configure(preferences, retain));
                }
                Some(Command::Cancel(id)) => {
                    if self
                        .current
                        .as_ref()
                        .is_some_and(|(current, _, _)| *current == id)
                    {
                        if let Some((_, speech, _)) = self.current.take() {
                            speech.cancellation()();
                        }
                        self.owner.cancel(id);
                        (self.events)(Event::Cancelled { id });
                    }
                }
                Some(Command::Shutdown) => break,
                None => {}
            }
            self.tick();
        }
        if let Some((_, speech, _)) = self.current.take() {
            speech.cancellation()();
        }
        self.owner.shutdown();
    }
}
