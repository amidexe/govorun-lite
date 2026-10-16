//! Pinned GigaAM/Silero inference. Capture feeds normalized PCM here; product
//! modes, delivery, statistics and network access stay in the shared owner.
use crate::pinned_asr::Session;
#[cfg(windows)]
use crate::segmentation::ShortUtterance;
use crate::{
    recognition::{Cancellation, Failure, Queue, Recognition},
    vad::Vad,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use transcribe_cpp::Backend;

#[derive(Clone)]
pub struct Recognizer {
    session: Arc<Mutex<Session>>,
}
impl Recognizer {
    /// Load the current packaged compute runtime on a blocking executor. Runtime
    /// is the exact CTranscribe framework binary/transcribe.dll, not a directory.
    pub fn load(path: &Path, runtime: &Path, backend: Backend) -> Result<Self, &'static str> {
        let session = Session::load(path, runtime, backend)?;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }
    pub fn backend(&self) -> String {
        self.session
            .lock()
            .map(|session| session.backend.clone())
            .unwrap_or_else(|_| "unavailable".into())
    }
    fn queue(&self) -> Queue {
        let session = self.session.clone();
        Queue::new(Cancellation::default(), move |pcm, token| {
            if token.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            let mut session = session.lock().map_err(|_| Failure::Decode)?;
            session.decode(pcm, token)
        })
    }
}

pub struct Recording {
    vad: Vad,
    queue: Queue,
    #[cfg(windows)]
    short: ShortUtterance,
    window: Vec<f32>,
}
/// A finished capture can leave its VAD on the owning thread and enqueue only
/// the decoder queue. No unsafe transfer of a native VAD pointer is required.
pub struct Job {
    queue: Queue,
}
impl Job {
    pub fn cancellation(&self) -> Cancellation {
        self.queue.cancellation()
    }
    pub fn cancel(&self) {
        self.queue.cancellation().cancel();
    }
    /// Run on a blocking executor after the preceding insertion has finished.
    pub fn finish(self) -> Result<Recognition, &'static str> {
        self.queue.complete().map_err(|error| match error {
            Failure::Cancelled => "recording_cancelled",
            _ => "recognition_worker_failed",
        })
    }
}
impl Recording {
    pub fn begin(
        recognizer: &Recognizer,
        model: &Path,
        runtime: &Path,
    ) -> Result<Self, &'static str> {
        let vad = Vad::new(model, runtime).map_err(|_| "recognition_vad_unavailable")?;
        Ok(Self {
            vad,
            queue: recognizer.queue(),
            #[cfg(windows)]
            short: ShortUtterance::default(),
            window: Vec::new(),
        })
    }
    pub fn cancellation(&self) -> Cancellation {
        self.queue.cancellation()
    }

    pub fn accept(&mut self, pcm: &[f32]) -> Result<(), &'static str> {
        if self.queue.cancellation().is_cancelled() {
            return Err("recording_cancelled");
        }
        if pcm.iter().any(|sample| !sample.is_finite()) {
            return Err("recording_invalid_pcm");
        }
        #[cfg(windows)]
        self.short.accept(pcm);
        self.window.extend_from_slice(pcm);
        let mut offset = 0;
        while self.window.len() - offset >= 512 {
            let segments = self
                .vad
                .accept(&self.window[offset..offset + 512])
                .map_err(|_| "recognition_vad_failed")?;
            #[cfg(windows)]
            self.short
                .detected(self.vad.speech_detected() || !segments.is_empty());
            self.enqueue(segments, cfg!(target_os = "macos"))?;
            offset += 512;
        }
        self.window.drain(..offset);
        Ok(())
    }

    pub fn finish(self) -> Result<Recognition, &'static str> {
        self.stop()?.finish()
    }

    pub fn stop(mut self) -> Result<Job, &'static str> {
        // Flush the final partial model window immediately, without dropping
        // the end of a word or waiting for a real-time pause.
        if !self.window.is_empty() {
            self.window.resize(512, 0.0);
            let segments = self
                .vad
                .accept(&self.window)
                .map_err(|_| "recognition_vad_failed")?;
            #[cfg(windows)]
            self.short
                .detected(self.vad.speech_detected() || !segments.is_empty());
            self.enqueue(segments, cfg!(target_os = "macos"))?;
        }
        let segments = self.vad.flush().map_err(|_| "recognition_vad_failed")?;
        #[cfg(windows)]
        self.short.detected(!segments.is_empty());
        self.enqueue(segments, cfg!(target_os = "macos"))?;
        #[cfg(windows)]
        {
            let segments = std::mem::take(&mut self.short).finish();
            self.enqueue(segments, false)?;
        }
        Ok(Job { queue: self.queue })
    }

    fn enqueue(&self, segments: Vec<Vec<f32>>, strict: bool) -> Result<(), &'static str> {
        for segment in segments {
            // macOS formerly rejects exactly 1600; Windows accepts it.
            if segment.len() > 1600 || (!strict && segment.len() == 1600) {
                self.queue
                    .enqueue(segment)
                    .map_err(|_| "recognition_queue_closed")?;
            }
        }
        Ok(())
    }
}

impl crate::capture::executor::Speech for Recording {
    type Job = Job;
    fn cancellation(&self) -> Arc<dyn Fn() + Send + Sync> {
        let cancellation = self.cancellation();
        Arc::new(move || cancellation.cancel())
    }
    fn accept(&mut self, pcm: &[f32]) -> Result<(), &'static str> {
        self.accept(pcm)
    }
    fn stop(self) -> Result<Job, &'static str> {
        self.stop()
    }
}
