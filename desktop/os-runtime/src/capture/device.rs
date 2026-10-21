//! Pinned Handy recorder owns device I/O, resampling and stop acknowledgement.
//! This adapter only delivers bounded 16 kHz mono PCM to the existing executor.
use super::{admission::Admission, error};
use govorun_core::journal_report::capture_failure::Operation;
use handy_recorder::{AudioChunk, Channels, EndReason, Recorder, RecorderConfig, Sink};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};

pub struct Input { pub id: String, pub name: String, pub channels: u16, pub is_default: bool }
fn stored_id(device: &handy_recorder::InputDevice) -> String {
    device.id.strip_prefix(&format!("{}:", device.backend)).unwrap_or(&device.id).into()
}
pub fn inputs() -> Result<Vec<Input>, &'static str> {
    handy_recorder::list_input_devices().map_err(|_| "microphone_enumeration_failed")?
        .into_iter().map(|device| Ok(Input { id: stored_id(&device), name: device.name,
            channels: device.channels.unwrap_or(1), is_default: device.is_default })).collect()
}
struct Queue { samples: Mutex<Vec<f32>>, failure: Mutex<Option<&'static str>> }
impl Queue {
    fn failed(&self, code: &'static str) {
        if let Ok(mut failure) = self.failure.lock() { failure.get_or_insert(code); }
    }
    fn check(&self) -> Result<(), &'static str> {
        self.failure.lock().map_err(|_| "microphone_callback_failed")?.map_or(Ok(()), Err)
    }
}
struct InputSink { queue: Arc<Queue>, admission: Arc<Admission> }
impl Sink for InputSink {
    fn process_chunk(&mut self, chunk: AudioChunk<'_>) {
        if self.admission.is_cancelled() || chunk.valid_frames == 0 { return; }
        if chunk.sample_rate != 16_000 || chunk.channels != 1 || chunk.valid_frames > chunk.samples.len()
            || chunk.samples[..chunk.valid_frames].iter().any(|sample| !sample.is_finite()) {
            self.queue.failed("microphone_packet_invalid"); return;
        }
        let Ok(mut pending) = self.queue.samples.lock() else {
            self.queue.failed("microphone_callback_failed"); return;
        };
        if pending.len().saturating_add(chunk.valid_frames) > 480_000 {
            self.queue.failed("microphone_buffer_overflow"); return;
        }
        pending.extend(chunk.samples[..chunk.valid_frames].iter().map(|s| s.clamp(-1.0, 1.0)));
        self.admission.mark_first_packet();
    }
}
struct Attempt { admission: Arc<Admission>, active: bool }
pub struct Session {
    recorder: Recorder<InputSink>, queue: Arc<Queue>, attempt: Mutex<Attempt>, completed: AtomicBool,
    pub device_id: String, pub used_system_fallback: bool,
}
impl Session {
    pub fn open(preferred: &str, channel: Option<usize>, admission: Arc<Admission>) -> Result<Self, &'static str> {
        let devices = handy_recorder::list_input_devices().map_err(|_| "microphone_enumeration_failed")?;
        let selected = devices.iter().find(|device| !preferred.is_empty() && stored_id(device) == preferred);
        let source = selected.or_else(|| devices.iter().find(|device| device.is_default));
        let config = RecorderConfig { device: selected.map(|device| device.id.clone()),
            channels: channel.filter(|index| source.and_then(|d| d.channels).is_none_or(|n| *index < n as usize))
                .and_then(|index| u16::try_from(index).ok()).map_or(Channels::MixToMono, Channels::Only),
            ..RecorderConfig::speech() };
        let fallback = selected.is_none() && !preferred.is_empty();
        let queue = Arc::new(Queue { samples: Mutex::new(Vec::new()), failure: Mutex::new(None) });
        let failed = queue.clone();
        let recorder = Recorder::open_with_failure_handler(config, move |error| {
            let token = Admission::new(None);
            failed.failed(error::recorder(&error, &token, Operation::Open));
        }).map_err(|error| error::recorder(&error, &admission, Operation::Open))?;
        admission.input_opened();
        let device_id = stored_id(&recorder.info().device);
        let session = Self { recorder, queue, attempt: Mutex::new(Attempt { admission: admission.clone(), active: false }),
            completed: AtomicBool::new(false), device_id, used_system_fallback: fallback };
        session.begin(admission)?;
        Ok(session)
    }
    pub fn begin(&self, admission: Arc<Admission>) -> Result<(), &'static str> {
        self.queue.check()?;
        if admission.is_cancelled() { return Err("recording_cancelled"); }
        self.queue.samples.lock().map_err(|_| "microphone_callback_failed")?.clear();
        self.completed.store(false, Ordering::Release);
        let active = admission.start_nanos().is_some();
        if active {
            self.recorder.start(InputSink { queue: self.queue.clone(), admission: admission.clone() })
                .map_err(|error| error::recorder(&error.error, &admission, Operation::Start))?;
        }
        *self.attempt.lock().map_err(|_| "microphone_callback_failed")? = Attempt { admission, active };
        Ok(())
    }
    fn finish_if_released(&self) -> Result<(), &'static str> {
        let token = {
            let mut attempt = self.attempt.lock().map_err(|_| "microphone_callback_failed")?;
            if !attempt.active || attempt.admission.deadline().is_none() { return Ok(()); }
            attempt.active = false; attempt.admission.clone()
        };
        // Handy owns the final callback block and drain. No timestamp gate or VAD wait.
        let stopped = self.recorder.stop().map_err(|error| error::recorder(&error, &token, Operation::Start))?;
        if stopped.dropped_frames != 0 { return Err("microphone_buffer_overflow"); }
        match stopped.end_reason {
            EndReason::StopCalled => (),
            EndReason::RecorderFailed(error) => return Err(error::recorder(&error, &token, Operation::Start)),
            EndReason::SinkPanicked(_) => return Err("microphone_callback_failed"),
        }
        self.completed.store(true, Ordering::Release); Ok(())
    }
    pub fn drain(&self) -> Result<Vec<f32>, &'static str> {
        self.finish_if_released()?; self.queue.check()?;
        if self.attempt.lock().map_err(|_| "microphone_callback_failed")?.admission.is_cancelled() {
            self.queue.samples.lock().map_err(|_| "microphone_callback_failed")?.clear(); return Ok(Vec::new());
        }
        Ok(std::mem::take(&mut *self.queue.samples.lock().map_err(|_| "microphone_callback_failed")?))
    }
    pub fn drained(&self) -> Result<bool, &'static str> {
        self.queue.check()?; Ok(self.completed.load(Ordering::Acquire))
    }
    pub fn idle(&self) -> Result<(), &'static str> { self.queue.check() }
    pub fn cancel(&self) {
        let active = self.attempt.lock().ok().map(|mut attempt| {
            attempt.admission.cancel(); let active = attempt.active; attempt.active = false; active
        }).unwrap_or(false);
        if active { let _ = self.recorder.stop(); }
        if let Ok(mut samples) = self.queue.samples.lock() { samples.clear(); }
        self.completed.store(true, Ordering::Release);
    }
}
