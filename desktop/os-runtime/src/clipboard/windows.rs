use std::{ptr, thread, time::Duration};
use windows_sys::{
    core::w,
    Win32::{
        Foundation::{GlobalFree, HANDLE, HWND},
        System::{DataExchange::*, Memory::*},
        UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow},
    },
};
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}
struct Allocation(HANDLE);
impl Drop for Allocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                GlobalFree(self.0);
            }
        }
    }
}
struct Open;
impl Drop for Open {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}
pub(super) fn copy(text: &str) -> Result<(), &'static str> {
    // Materialize Unicode before replacing the clipboard. The temporary hidden
    // owner never activates a window and gives a new writer its own identity.
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let allocation = unsafe { GlobalAlloc(GMEM_MOVEABLE, text.len() * 2) };
    if allocation.is_null() {
        return Err("clipboard_allocation_failed");
    }
    let mut allocation = Allocation(allocation);
    let address = unsafe { GlobalLock(allocation.0) };
    if address.is_null() {
        return Err("clipboard_allocation_inaccessible");
    }
    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), address.cast::<u16>(), text.len());
        GlobalUnlock(allocation.0);
    }
    let window = unsafe {
        CreateWindowExW(
            0,
            w!("STATIC"),
            w!("Govorun report clipboard"),
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        )
    };
    if window.is_null() {
        return Err("clipboard_owner_unavailable");
    }
    let window = Window(window);
    let mut opened = false;
    for attempt in 0..5 {
        if unsafe { OpenClipboard(window.0) } != 0 {
            opened = true;
            break;
        }
        if attempt != 4 {
            thread::sleep(Duration::from_millis(25 * (attempt + 1)));
        }
    }
    if !opened {
        return Err("clipboard_unavailable");
    }
    let _open = Open;
    if unsafe { EmptyClipboard() } == 0 {
        return Err("clipboard_clear_failed");
    }
    // CF_UNICODETEXT = 13; successful SetClipboardData transfers ownership.
    if unsafe { SetClipboardData(13, allocation.0) }.is_null() {
        return Err("clipboard_write_failed");
    }
    allocation.0 = ptr::null_mut();
    Ok(())
}
