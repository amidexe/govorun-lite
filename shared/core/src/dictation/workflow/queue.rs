//! Ordered attempts; cancellation reaches audio jobs and skips waiting work.
use govorun_core::dictation_jobs::{JobId, Jobs};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::{sync::oneshot, task::JoinHandle};
#[derive(Clone, Default)]
pub struct Cancellation(Arc<Signal>);
#[derive(Default)]
struct Signal {
    flag: AtomicBool,
    changed: tokio::sync::Notify,
}
impl Cancellation {
    pub fn is_cancelled(&self) -> bool {
        self.0.flag.load(Ordering::Acquire)
    }
    fn cancel(&self) {
        self.0.flag.store(true, Ordering::Release);
        self.0.changed.notify_waiters();
    }
    pub async fn cancelled(&self) {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            changed.await;
        }
    }
}
struct Entry<J> {
    job: Arc<J>,
    cancellation: Cancellation,
}
struct State<J> {
    jobs: Jobs,
    entries: BTreeMap<JobId, Entry<J>>,
    tail: Option<JoinHandle<()>>,
}
pub struct Queue<J> {
    state: Mutex<State<J>>,
    cancel_job: Arc<dyn Fn(&J) + Send + Sync>,
    // A sink only, without re-entry. The gate keeps publication order stable.
    count_changed: Arc<dyn Fn(usize) + Send + Sync>,
}
pub struct Completion(oneshot::Receiver<Result<(), &'static str>>);
impl Completion {
    pub async fn wait(self) -> Result<(), &'static str> {
        self.0.await.map_err(|_| "dictation_queue_stopped")?
    }
}
impl<J: Send + Sync + 'static> Queue<J> {
    pub fn new(
        cancel_job: Arc<dyn Fn(&J) + Send + Sync>,
        count_changed: Arc<dyn Fn(usize) + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                jobs: Jobs::default(),
                entries: BTreeMap::new(),
                tail: None,
            }),
            cancel_job,
            count_changed,
        })
    }
    pub fn count(&self) -> usize {
        self.state.lock().unwrap().jobs.count()
    }
    pub fn enqueue<F, W>(self: &Arc<Self>, job: J, process: F) -> Result<Completion, &'static str>
    where
        F: FnOnce(Arc<J>, Cancellation) -> W + Send + 'static,
        W: Future<Output = ()> + Send + 'static,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "dictation_queue_unavailable")?;
        let id = state
            .jobs
            .enqueue()
            .map_err(|_| "dictation_queue_exhausted")?;
        let job = Arc::new(job);
        let cancellation = Cancellation::default();
        state.entries.insert(
            id,
            Entry {
                job: job.clone(),
                cancellation: cancellation.clone(),
            },
        );
        (self.count_changed)(state.jobs.count());
        let previous = state.tail.take();
        let owner = self.clone();
        let (completed, receive) = oneshot::channel();
        state.tail = Some(tokio::spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let result = if cancellation.is_cancelled() {
                Ok(())
            } else {
                // A panicking effect must not leave an immortal UI count.
                tokio::spawn(async move { process(job, cancellation).await })
                    .await
                    .map_err(|_| "dictation_job_panicked")
            };
            owner.finish(id);
            let _ = completed.send(result);
        }));
        Ok(Completion(receive))
    }
    pub fn cancel_all(&self) {
        let cancelled = {
            let mut state = self.state.lock().unwrap();
            let entries = std::mem::take(&mut state.entries);
            for entry in entries.values() {
                entry.cancellation.cancel();
            }
            state.jobs.cancel_all();
            // Detach instead of aborting durable finalization already admitted
            // by another feature. Late completion cannot mutate a new queue.
            state.tail = None;
            (self.count_changed)(0);
            entries
        };
        for entry in cancelled.values() {
            (self.cancel_job)(&entry.job);
        }
    }
    fn finish(&self, id: JobId) {
        let mut state = self.state.lock().unwrap();
        if !state.jobs.complete(id) {
            return;
        }
        state.entries.remove(&id);
        if state.jobs.count() == 0 {
            state.tail = None;
        }
        (self.count_changed)(state.jobs.count());
    }
}
