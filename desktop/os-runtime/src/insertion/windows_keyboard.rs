//! Physical VK_V, never Unicode typing. Release survives a partial SDK batch.
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};
fn key(code: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: crate::PASTE_EVENT_TAG,
            },
        },
    }
}
fn send(keys: &[INPUT]) -> bool {
    unsafe {
        SendInput(
            keys.len() as u32,
            keys.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        ) == keys.len() as u32
    }
}
pub fn press() -> bool {
    send(&[key(0x11, false), key(0x56, false), key(0x56, true)])
}
pub fn release(pressed: bool) -> bool {
    if pressed {
        send(&[key(0x11, true)])
    } else {
        send(&[key(0x56, true), key(0x11, true)])
    }
}
