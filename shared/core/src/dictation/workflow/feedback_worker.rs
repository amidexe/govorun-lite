//! Serialized media effects. OS handles never leave this worker's thread.
#[path = "feedback.rs"]
mod feedback;
use feedback::{Effect, State};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub trait Output: 'static {
    fn cue(&mut self, starting: bool) -> Result<(), &'static str>;
    fn mute(&mut self) -> Result<(), &'static str>;
    fn restore(&mut self) -> Result<(), &'static str>;
}
enum Command {
    Effect(Effect),
    Shutdown(mpsc::Sender<()>),
}
struct Shared {
    state: Mutex<State>,
    send: mpsc::Sender<Command>,
}
impl Shared {
    fn dispatch(&self, effects: Vec<Effect>) {
        for effect in effects {
            if let Effect::Mute(id, delay) = effect {
                if !delay.is_zero() {
                    let send = self.send.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(delay);
                        let _ = send.send(Command::Effect(Effect::Mute(id, Duration::ZERO)));
                    });
                    continue;
                }
            }
            let _ = self.send.send(Command::Effect(effect));
        }
    }
    fn cue_allowed(&self, id: u64, starting: bool) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.cue_allowed(id, starting))
    }
    fn eligible(&self, id: u64) -> bool {
        self.state.lock().is_ok_and(|state| state.owns_mute(id))
    }
}
pub struct Worker {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}
impl Worker {
    pub fn new<O: Output>(
        delay: Duration,
        admission: bool,
        factory: impl FnOnce() -> O + Send + 'static,
        error: Arc<dyn Fn(&'static str) + Send + Sync>,
    ) -> Self {
        let (send, receive) = mpsc::channel();
        let shared = Arc::new(Shared {
            state: Mutex::new(State::new(delay, admission)),
            send,
        });
        let owned = shared.clone();
        let thread = std::thread::spawn(move || {
            let mut output = factory();
            while let Ok(command) = receive.recv() {
                let result = match command {
                    Command::Effect(Effect::StartCue(id)) => {
                        let result = if owned.cue_allowed(id, true) {
                            output.cue(true)
                        } else {
                            Ok(())
                        };
                        // Completion is published only after synchronous playback returns.
                        if let Ok(mut state) = owned.state.lock() {
                            owned.dispatch(state.cue_finished(id, Instant::now()));
                        }
                        result
                    }
                    Command::Effect(Effect::StopCue(id)) => {
                        if owned.cue_allowed(id, false) {
                            output.cue(false)
                        } else {
                            Ok(())
                        }
                    }
                    Command::Effect(Effect::Mute(id, _)) => {
                        if owned.eligible(id) {
                            let result = output.mute();
                            // Cancellation during SetMute must restore the owned change too.
                            if !owned.eligible(id) {
                                if let Err(code) = output.restore() {
                                    error(code);
                                }
                            }
                            result
                        } else {
                            Ok(())
                        }
                    }
                    Command::Effect(Effect::Restore) => output.restore(),
                    Command::Shutdown(done) => {
                        if let Err(code) = output.restore() {
                            error(code);
                        }
                        // Drop resources before acknowledging complete shutdown.
                        drop(output);
                        let _ = done.send(());
                        return;
                    }
                };
                if let Err(code) = result {
                    error(code);
                }
            }
            if let Err(code) = output.restore() {
                error(code);
            }
        });
        Self {
            shared,
            thread: Mutex::new(Some(thread)),
        }
    }
    fn change(&self, change: impl FnOnce(&mut State) -> Vec<Effect>) {
        if let Ok(mut state) = self.shared.state.lock() {
            self.shared.dispatch(change(&mut state));
        }
    }
    pub fn start(&self, id: u64, play: bool) {
        self.change(|state| state.start(id, play));
    }
    pub fn ready(&self, id: u64, play: bool, mute: bool) {
        self.change(|state| state.ready(id, play, mute, Instant::now()));
    }
    pub fn finish(&self, id: u64, play: bool) {
        self.change(|state| state.finish(id, play));
    }
    pub fn cancel(&self) {
        self.change(State::cancel);
    }
    pub fn shutdown(&self) {
        let Ok(mut thread) = self.thread.lock() else {
            return;
        };
        let Some(join) = thread.take() else { return };
        self.cancel();
        let (done, wait) = mpsc::channel();
        let _ = self.shared.send.send(Command::Shutdown(done));
        let _ = wait.recv();
        let _ = join.join();
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.shutdown();
    }
}
