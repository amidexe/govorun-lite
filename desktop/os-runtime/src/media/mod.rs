//! Owned output-device mute. No microphone, player discovery or user storage.
pub trait Port {
    type Device: Clone;
    fn device(&mut self) -> Result<Self::Device, &'static str>;
    fn exists(&self, device: &Self::Device) -> bool;
    fn muted(&self, device: &Self::Device) -> Result<bool, &'static str>;
    fn set(&self, device: &Self::Device, muted: bool) -> Result<(), &'static str>;
}
pub struct Mute<P: Port> {
    changed: Option<P::Device>,
    port: P,
}
impl<P: Port> Mute<P> {
    pub fn new(port: P) -> Self {
        Self {
            port,
            changed: None,
        }
    }
    pub fn mute(&mut self) -> Result<(), &'static str> {
        if self.changed.is_some() {
            return Ok(());
        }
        let device = self.port.device()?;
        if self.port.muted(&device)? {
            return Ok(());
        }
        self.changed = Some(device.clone());
        if let Err(error) = self.port.set(&device, true) {
            self.restore()?;
            return Err(error);
        }
        Ok(())
    }
    pub fn restore(&mut self) -> Result<(), &'static str> {
        let Some(device) = &self.changed else {
            return Ok(());
        };
        // Restore the captured endpoint, even if the default or preference
        // changed. An unplugged endpoint or a user's unmute needs no write.
        if self.port.exists(device) && self.port.muted(device)? {
            self.port.set(device, false)?;
        }
        self.changed = None;
        Ok(())
    }
}
impl<P: Port> Drop for Mute<P> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{Cue, Output};
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::{Cue, Output};

pub const START_WAVE: &[u8] = include_bytes!("../../assets/recording-start.wav");
pub const STOP_WAVE: &[u8] = include_bytes!("../../assets/recording-stop.wav");
