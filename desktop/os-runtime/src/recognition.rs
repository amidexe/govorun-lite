//! Each capture owns its queue and cancellation token. Native decode may end
//! late; cancelled text is never returned, and another queue is independent.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub use govorun_core::features::blocking_cancellation::Cancellation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Cancelled,
    Backend,
    Decode,
    Closed,
    WorkerPanicked,
}

/// Text stays private to the dictation executor. No Debug/Serialize projection
/// can accidentally place it in a renderer snapshot or diagnostic log.
pub struct Recognition {
    pub text: String,
    pub fragments: Vec<String>,
    pub speech_samples: usize,
    pub chunks: usize,
    pub failed_chunks: usize,
    pub decode_duration: Duration,
}

pub struct Queue {
    sender: Option<mpsc::Sender<Vec<f32>>>,
    worker: Option<JoinHandle<Result<Recognition, Failure>>>,
    cancellation: Cancellation,
}

impl Queue {
    pub fn new(
        cancellation: Cancellation,
        mut decode: impl FnMut(&[f32], &Cancellation) -> Result<String, Failure> + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Vec<f32>>();
        let token = cancellation.clone();
        let worker = thread::spawn(move || {
            let mut text = Vec::new();
            let mut samples = 0;
            let mut chunks = 0;
            let mut failed = 0;
            let mut elapsed = Duration::ZERO;
            for pcm in receiver {
                if token.is_cancelled() {
                    continue;
                }
                chunks += 1;
                samples += pcm.len();
                let started = Instant::now();
                let result = decode(&pcm, &token);
                elapsed += started.elapsed();
                if token.is_cancelled() {
                    continue;
                }
                match result {
                    Ok(value) if !value.trim().is_empty() => text.push(value.trim().to_owned()),
                    Ok(_) => {}
                    Err(Failure::Cancelled) => token.cancel(),
                    Err(_) => failed += 1,
                }
            }
            if token.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            Ok(Recognition {
                text: text.join(" "),
                fragments: text,
                speech_samples: samples,
                chunks,
                failed_chunks: failed,
                decode_duration: elapsed,
            })
        });
        Self {
            sender: Some(sender),
            worker: Some(worker),
            cancellation,
        }
    }

    pub fn enqueue(&self, samples: Vec<f32>) -> Result<(), Failure> {
        if self.cancellation.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        self.sender
            .as_ref()
            .ok_or(Failure::Closed)?
            .send(samples)
            .map_err(|_| Failure::Closed)
    }

    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }

    /// Run on a blocking executor, never on the renderer/main thread. Closing
    /// input drains all submitted segments before delivering ordered text.
    pub fn complete(mut self) -> Result<Recognition, Failure> {
        self.sender.take();
        self.worker
            .take()
            .ok_or(Failure::Closed)?
            .join()
            .map_err(|_| Failure::WorkerPanicked)?
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        if self.worker.is_some() {
            self.cancellation.cancel();
        }
        self.sender.take();
        // The worker keeps native resources alive until a delayed abort ends;
        // the caller must not wait for native work on the UI thread.
    }
}
