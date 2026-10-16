//! ServiceManagement calls must execute on the application's main thread.
use super::login::{self, Login};
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool},
};
#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}
fn class() -> Result<&'static AnyClass, &'static str> {
    AnyClass::get(c"SMAppService").ok_or("login_registration_unavailable")
}
fn service() -> Result<Retained<AnyObject>, &'static str> {
    let raw: *mut AnyObject = unsafe { msg_send![class()?, mainAppService] };
    unsafe { Retained::retain(raw) }.ok_or("login_registration_unavailable")
}
pub fn login_status() -> Login {
    let Ok(service) = service() else {
        return Login::Unavailable;
    };
    let status: isize = unsafe { msg_send![&*service, status] };
    login::macos_status(status)
}
pub fn set_login(enabled: bool) -> Result<(), &'static str> {
    let service = service()?;
    let mut error: *mut AnyObject = std::ptr::null_mut();
    let accepted: Bool = if enabled {
        unsafe { msg_send![&*service, registerAndReturnError: &mut error] }
    } else {
        unsafe { msg_send![&*service, unregisterAndReturnError: &mut error] }
    };
    if accepted.as_bool() {
        Ok(())
    } else {
        Err("login_registration_failed")
    }
}
pub fn open_login_settings() -> Result<(), &'static str> {
    let _: () = unsafe { msg_send![class()?, openSystemSettingsLoginItems] };
    Ok(())
}
