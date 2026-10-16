//! One physical stream owner, run on its blocking capture executor. Durable
//! preferences are inputs only: a missing endpoint never overwrites them.
use super::admission::Admission;
use std::sync::Arc;

pub use govorun_core::features::dictation::inputs::Preferences;

pub trait Device {
    fn id(&self) -> &str;
    fn begin(&self, admission: Arc<Admission>) -> Result<(), &'static str>;
    fn drain(&self) -> Result<Vec<f32>, &'static str>;
    fn drained(&self) -> Result<bool, &'static str>;
    fn cancel(&self);
    fn idle(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

pub trait Driver {
    type Device: Device;
    /// Resolve on each admission, including when a warm stream exists.
    fn resolve(&self, preferences: &Preferences) -> Result<String, &'static str>;
    /// Arm admission before starting the device; first synchronous packets
    /// belong to this recording, and cancellation may arrive during open.
    fn open(
        &mut self,
        preferences: &Preferences,
        generation: u64,
        admission: Arc<Admission>,
    ) -> Result<Self::Device, &'static str>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Capturing,
    Complete,
    Cancelled,
}
// Ephemeral PCM is local to the executor. No Debug/Serialize implementation.
pub struct Packet {
    pub pcm: Vec<f32>,
    pub state: State,
}
struct Active {
    id: u64,
    admission: Arc<Admission>,
    opened: std::time::Instant,
}
pub struct Owner<D: Driver> {
    driver: D,
    device: Option<D::Device>,
    opened: Option<Preferences>,
    desired: Preferences,
    active: Option<Active>,
    next_stream: u64,
}

impl<D: Driver> Owner<D> {
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            device: None,
            opened: None,
            desired: Preferences::default(),
            active: None,
            next_stream: 0,
        }
    }

    pub fn preferences(&self) -> &Preferences {
        &self.desired
    }

    /// The host commits the preference only after success. Changes while
    /// capturing are deferred to the next admission, preserving its source.
    pub fn configure(
        &mut self,
        preferences: Preferences,
        retain_on_failure: bool,
    ) -> Result<(), &'static str> {
        let previous = std::mem::replace(&mut self.desired, preferences.clone());
        if self.active.is_some() {
            return Ok(());
        }
        if !preferences.warm {
            self.close();
            return Ok(());
        }
        match self.open(&preferences, Admission::new(None)) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.close();
                if !retain_on_failure {
                    self.desired = previous;
                }
                Err(error)
            }
        }
    }

    pub fn begin(
        &mut self,
        id: u64,
        admission: Arc<Admission>,
        preferences: Preferences,
    ) -> Result<(), &'static str> {
        if self.active.is_some() {
            return Err("recording_already_active");
        }
        if admission.is_cancelled() {
            return Err("recording_cancelled");
        }
        self.desired = preferences.clone();
        self.active = Some(Active {
            id,
            admission: admission.clone(),
            opened: std::time::Instant::now(),
        });
        if let Err(error) = self.open(&preferences, admission.clone()) {
            self.active = None;
            self.close();
            return Err(error);
        }
        if admission.is_cancelled() {
            self.cancel(id);
            return Err("recording_cancelled");
        }
        // Model/device creation is complete. Bound the wait for this attempt's
        // first packet, including on a reused warm device, independently of
        // the stop-tail deadline and the previous attempt's audio.
        self.active.as_mut().unwrap().opened = std::time::Instant::now();
        Ok(())
    }

    fn open(
        &mut self,
        preferences: &Preferences,
        admission: Arc<Admission>,
    ) -> Result<(), &'static str> {
        let resolved = self.driver.resolve(preferences)?;
        let reuse = self
            .device
            .as_ref()
            .is_some_and(|device| device.id() == resolved)
            && self.opened.as_ref() == Some(preferences);
        if reuse {
            if let Some(device) = &self.device {
                if device.begin(admission.clone()).is_ok() {
                    return Ok(());
                }
            }
            // An idle disconnect is retried on the next admission. Its old
            // callback/error cannot be attributed to this new recording.
        }
        self.close();
        self.next_stream = self
            .next_stream
            .checked_add(1)
            .ok_or("microphone_generation_exhausted")?;
        let device = self
            .driver
            .open(preferences, self.next_stream, admission.clone())?;
        if admission.is_cancelled() {
            device.cancel();
            return Err("recording_cancelled");
        }
        self.device = Some(device);
        self.opened = Some(preferences.clone());
        Ok(())
    }

    /// Poll frequently enough to drain the bounded callback buffer. The
    /// exact tail comes from admission; timeout cannot extend captured audio.
    pub fn poll(&mut self, now: u64) -> Result<Packet, &'static str> {
        let Some(active) = &self.active else {
            if self
                .device
                .as_ref()
                .is_some_and(|device| device.idle().is_err())
            {
                self.close();
            }
            return Ok(Packet {
                pcm: Vec::new(),
                state: State::Idle,
            });
        };
        let id = active.id;
        let admission = active.admission.clone();
        if admission.is_cancelled() {
            self.cancel(id);
            return Ok(Packet {
                pcm: Vec::new(),
                state: State::Cancelled,
            });
        }
        let result = (|| {
            let device = self.device.as_ref().ok_or("microphone_unavailable")?;
            let pcm = device.drain()?;
            let drained = device.drained()?;
            if !drained
                && admission.first_packet_at().is_none()
                && active.opened.elapsed() >= std::time::Duration::from_secs(5)
            {
                return Err("microphone_timeout");
            }
            if !drained
                && admission
                    .deadline()
                    .is_some_and(|end| now.saturating_sub(end) >= 3_000_000_000)
            {
                return Err("microphone_timeout");
            }
            Ok((pcm, drained))
        })();
        if admission.is_cancelled() {
            self.cancel(id);
            return Ok(Packet {
                pcm: Vec::new(),
                state: State::Cancelled,
            });
        }
        let (pcm, drained) = match result {
            Ok(value) => value,
            Err(error) => {
                self.active = None;
                self.close();
                return Err(error);
            }
        };
        if drained {
            self.active = None;
            if !self.desired.warm {
                self.close();
            }
        }
        Ok(Packet {
            pcm,
            state: if drained {
                State::Complete
            } else {
                State::Capturing
            },
        })
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        if !self.active.as_ref().is_some_and(|active| active.id == id) {
            return false;
        }
        if let Some(active) = self.active.take() {
            active.admission.cancel();
        }
        if let Some(device) = &self.device {
            device.cancel();
        }
        if !self.desired.warm {
            self.close();
        }
        true
    }

    pub fn shutdown(&mut self) {
        if let Some(active) = self.active.take() {
            active.admission.cancel();
        }
        self.close();
    }

    fn close(&mut self) {
        // Dropping closes the stream. A successful drained admission must not
        // be turned into cancellation while its recognition job is queued.
        self.device.take();
        self.opened = None;
    }
}

impl<D: Driver> Drop for Owner<D> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
