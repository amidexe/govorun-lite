//! COM resources are created, used and dropped on the media worker apartment.
use super::Port;
use windows::Win32::{
    Media::Audio::{
        eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    },
    System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    },
};
#[derive(Clone)]
pub struct Device(IAudioEndpointVolume);
pub struct Output {
    initialized: bool,
}
impl Output {
    pub fn new() -> Result<Self, &'static str> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|_| "media_output_unavailable")?;
        Ok(Self { initialized: true })
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}
impl Port for Output {
    type Device = Device;
    fn device(&mut self) -> Result<Device, &'static str> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|_| "media_output_unavailable")?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|_| "media_output_unavailable")?;
            Ok(Device(
                device
                    .Activate(CLSCTX_ALL, None)
                    .map_err(|_| "media_output_unavailable")?,
            ))
        }
    }
    fn exists(&self, device: &Device) -> bool {
        match unsafe { device.0.GetMute() } {
            Ok(_) => true,
            Err(error) => !matches!(error.code().0 as u32, 0x88890004 | 0x80010108),
        }
    }
    fn muted(&self, device: &Device) -> Result<bool, &'static str> {
        unsafe { device.0.GetMute() }
            .map(|value| value.as_bool())
            .map_err(|_| "media_output_unavailable")
    }
    fn set(&self, device: &Device, muted: bool) -> Result<(), &'static str> {
        unsafe { device.0.SetMute(muted, std::ptr::null()) }.map_err(|_| "media_output_unavailable")
    }
}
#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(sound: *const u16, module: isize, flags: u32) -> i32;
}
pub struct Cue;
impl Cue {
    pub fn new() -> Self {
        Self
    }
    pub fn play(&mut self, starting: bool) -> Result<(), &'static str> {
        let wave = if starting {
            super::START_WAVE
        } else {
            super::STOP_WAVE
        };
        // Aligned little-endian storage remains alive through synchronous output.
        let wave: Vec<u16> = wave
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        if unsafe { PlaySoundW(wave.as_ptr(), 0, 0x0004 | 0x0002) } == 0 {
            Err("sound_cue_unavailable")
        } else {
            Ok(())
        }
    }
}
