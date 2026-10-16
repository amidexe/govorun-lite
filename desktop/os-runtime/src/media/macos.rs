//! CoreAudio retains the original device identity; NSSound stays on one worker.
use super::Port;
use objc2::rc::Retained;
use objc2_app_kit::NSSound;
use objc2_foundation::NSString;
#[repr(C)]
struct Address {
    selector: u32,
    scope: u32,
    element: u32,
}
fn address(selector: &[u8; 4], scope: &[u8; 4]) -> Address {
    Address {
        selector: u32::from_be_bytes(*selector),
        scope: u32::from_be_bytes(*scope),
        element: 0,
    }
}
#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(
        id: u32,
        address: *const Address,
        qualifier_size: u32,
        qualifier: *const std::ffi::c_void,
        size: *mut u32,
        data: *mut std::ffi::c_void,
    ) -> i32;
    fn AudioObjectSetPropertyData(
        id: u32,
        address: *const Address,
        qualifier_size: u32,
        qualifier: *const std::ffi::c_void,
        size: u32,
        data: *const std::ffi::c_void,
    ) -> i32;
    fn AudioObjectHasProperty(id: u32, address: *const Address) -> u8;
    fn AudioObjectIsPropertySettable(id: u32, address: *const Address, value: *mut u8) -> i32;
}
#[derive(Clone)]
pub struct Device {
    id: u32,
    uid: String,
}
pub struct Output;
impl Output {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self)
    }
}
fn number(id: u32, address: Address) -> Result<u32, &'static str> {
    let mut value = 0u32;
    let mut size = 4;
    if unsafe {
        AudioObjectGetPropertyData(
            id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            (&mut value as *mut u32).cast(),
        )
    } != 0
        || size != 4
    {
        return Err("media_output_unavailable");
    }
    Ok(value)
}
fn uid(id: u32) -> Result<String, &'static str> {
    let mut value: *mut NSString = std::ptr::null_mut();
    let mut size = std::mem::size_of::<*mut NSString>() as u32;
    if unsafe {
        AudioObjectGetPropertyData(
            id,
            &address(b"uid ", b"glob"),
            0,
            std::ptr::null(),
            &mut size,
            (&mut value as *mut *mut NSString).cast(),
        )
    } != 0
        || size != std::mem::size_of::<*mut NSString>() as u32
    {
        return Err("media_output_unavailable");
    }
    // CoreAudio's UID property returns a caller-owned CFString (NSString bridge).
    let value = unsafe { Retained::from_raw(value) }.ok_or("media_output_unavailable")?;
    if value.length() == 0 || value.length() > 1024 {
        return Err("media_output_unavailable");
    }
    Ok(value.to_string())
}
impl Port for Output {
    type Device = Device;
    fn device(&mut self) -> Result<Device, &'static str> {
        let id = number(1, address(b"dOut", b"glob"))?;
        Ok(Device { id, uid: uid(id)? })
    }
    fn exists(&self, device: &Device) -> bool {
        (unsafe { AudioObjectHasProperty(device.id, &address(b"uid ", b"glob")) }) != 0
            && uid(device.id).map_or(true, |value| value == device.uid)
    }
    fn muted(&self, device: &Device) -> Result<bool, &'static str> {
        Ok(number(device.id, address(b"mute", b"outp"))? != 0)
    }
    fn set(&self, device: &Device, muted: bool) -> Result<(), &'static str> {
        let address = address(b"mute", b"outp");
        let mut settable = 0;
        let value = u32::from(muted);
        if unsafe { AudioObjectHasProperty(device.id, &address) } == 0
            || unsafe { AudioObjectIsPropertySettable(device.id, &address, &mut settable) } != 0
            || settable == 0
        {
            return Err("media_output_unavailable");
        }
        if unsafe {
            AudioObjectSetPropertyData(
                device.id,
                &address,
                0,
                std::ptr::null(),
                4,
                (&value as *const u32).cast(),
            )
        } != 0
        {
            return Err("media_output_unavailable");
        }
        Ok(())
    }
}
pub struct Cue {
    sound: Option<Retained<NSSound>>,
}
impl Cue {
    pub fn new() -> Self {
        Self { sound: None }
    }
    pub fn play(&mut self, starting: bool) -> Result<(), &'static str> {
        if self.sound.is_none() {
            self.sound = NSSound::soundNamed(&NSString::from_str("Tink"));
        }
        let sound = self.sound.as_ref().ok_or("sound_cue_unavailable")?;
        sound.setVolume(0.7);
        let click = || {
            sound.stop();
            if !sound.play() {
                return Err("sound_cue_unavailable");
            }
            std::thread::sleep(std::time::Duration::from_millis(140));
            sound.stop();
            Ok(())
        };
        click()?;
        if !starting {
            std::thread::sleep(std::time::Duration::from_millis(30));
            click()?;
        }
        Ok(())
    }
}
