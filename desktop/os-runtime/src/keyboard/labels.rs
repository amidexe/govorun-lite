//! A physical key label for settings only, never an input-event text payload.
#[cfg(target_os = "macos")]
pub fn key_label(code: u16) -> Option<String> {
    // HIToolbox aborts if a UI process calls TIS from background threads.
    // Background facts/capture use the caller's existing physical-key fallback.
    let _main = objc2::MainThreadMarker::new()?;
    use std::ffi::c_void;
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn TISCopyCurrentKeyboardInputSource() -> *const c_void;
        fn TISGetInputSourceProperty(source: *const c_void, key: *const c_void) -> *const c_void;
        static kTISPropertyUnicodeKeyLayoutData: *const c_void;
        fn LMGetKbdType() -> u8;
        fn UCKeyTranslate(
            layout: *const c_void,
            code: u16,
            action: u16,
            modifiers: u32,
            keyboard: u32,
            options: u32,
            dead: *mut u32,
            max: usize,
            count: *mut usize,
            chars: *mut u16,
        ) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDataGetBytePtr(data: *const c_void) -> *const c_void;
        fn CFRelease(value: *const c_void);
    }
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        struct Source(*const c_void);
        impl Drop for Source {
            fn drop(&mut self) {
                unsafe { CFRelease(self.0) }
            }
        }
        let source = Source(source);
        let data = TISGetInputSourceProperty(source.0, kTISPropertyUnicodeKeyLayoutData);
        if data.is_null() {
            return None;
        }
        let bytes = CFDataGetBytePtr(data);
        if bytes.is_null() {
            return None;
        }
        let mut chars = [0u16; 4];
        let mut count = 0;
        let mut dead = 0;
        if UCKeyTranslate(
            bytes,
            code,
            3,
            0,
            LMGetKbdType() as u32,
            1,
            &mut dead,
            chars.len(),
            &mut count,
            chars.as_mut_ptr(),
        ) != 0
            || count == 0
            || count > chars.len()
        {
            return None;
        }
        let value = String::from_utf16(&chars[..count]).ok()?.to_uppercase();
        if value.len() > 64 || value.chars().any(char::is_control) {
            None
        } else {
            Some(value)
        }
    }
}
#[cfg(not(target_os = "macos"))]
pub fn key_label(_: u16) -> Option<String> {
    None
}
