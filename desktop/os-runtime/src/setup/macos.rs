//! AVFoundation facts and a single request owned by the Rust host identity.
use super::access::{self, Microphone};
use block2::RcBlock;
use objc2::{
    msg_send,
    runtime::{AnyClass, Bool},
};
use objc2_foundation::NSString;
use std::{ffi::c_void, sync::Arc};
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVMediaTypeAudio: *const c_void;
}
static REQUEST: access::Request = access::Request::new();
fn device() -> Option<&'static AnyClass> {
    AnyClass::get(c"AVCaptureDevice")
}
fn audio() -> Option<&'static NSString> {
    unsafe { (AVMediaTypeAudio as *const NSString).as_ref() }
}
pub fn microphone() -> Microphone {
    let (Some(device), Some(audio)) = (device(), audio()) else {
        return Microphone::Unknown;
    };
    let status: isize = unsafe { msg_send![device, authorizationStatusForMediaType: audio] };
    access::macos(status)
}
/// Called only for the existing user-selected permission command. The OS owns
/// its sheet; dropping the command never produces a second concurrent request.
pub fn request_microphone(completed: Arc<dyn Fn() + Send + Sync>) -> Result<bool, &'static str> {
    if microphone() != Microphone::NotDetermined {
        return Ok(false);
    }
    let device = device().ok_or("microphone_permission_unavailable")?;
    let audio = audio().ok_or("microphone_permission_unavailable")?;
    if !REQUEST.begin(Microphone::NotDetermined) {
        return Ok(true);
    }
    let callback = RcBlock::new(move |_allowed: Bool| {
        REQUEST.finish();
        completed();
    });
    let _: () = unsafe {
        msg_send![device, requestAccessForMediaType: audio, completionHandler: &*callback]
    };
    Ok(true)
}
