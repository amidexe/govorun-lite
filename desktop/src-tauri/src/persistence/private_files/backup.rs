//! Use Apple's app-data backup exclusion, including Time Machine/iCloud backup.
use super::Error;
use std::{ffi::c_void, os::unix::ffi::OsStrExt, path::Path, ptr};
type Ref = *const c_void;
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFURLIsExcludedFromBackupKey: Ref;
    static kCFBooleanTrue: Ref;
    fn CFURLCreateFromFileSystemRepresentation(allocator: Ref, bytes: *const u8, length: isize, directory: u8) -> Ref;
    fn CFURLSetResourcePropertyForKey(url: Ref, key: Ref, value: Ref, error: *mut Ref) -> u8;
    fn CFURLCopyResourcePropertyForKey(url: Ref, key: Ref, value: *mut Ref, error: *mut Ref) -> u8;
    fn CFEqual(a: Ref, b: Ref) -> u8;
    fn CFRelease(value: Ref);
}
struct Owned(Ref);
impl Drop for Owned { fn drop(&mut self) { if !self.0.is_null() { unsafe { CFRelease(self.0) }; } } }
fn excluded(url: Ref) -> bool {
    let mut value = ptr::null();
    let mut error = ptr::null();
    // Copy returns owned references. Neither errors nor paths/content are logged.
    let success = unsafe { CFURLCopyResourcePropertyForKey(url, kCFURLIsExcludedFromBackupKey, &mut value, &mut error) };
    let value = Owned(value);
    let _error = Owned(error);
    success != 0 && !value.0.is_null() && unsafe { CFEqual(value.0, kCFBooleanTrue) } != 0
}
pub(super) fn exclude(path: &Path, directory: bool) -> Result<(), Error> {
    let bytes = path.as_os_str().as_bytes();
    // The URL represents a checked regular file/private directory, not content.
    let url = Owned(unsafe { CFURLCreateFromFileSystemRepresentation(ptr::null(), bytes.as_ptr(), bytes.len() as isize, u8::from(directory)) });
    if url.0.is_null() { return Err(Error::Unavailable); }
    if excluded(url.0) { return Ok(()); }
    let mut error = ptr::null();
    let success = unsafe { CFURLSetResourcePropertyForKey(url.0, kCFURLIsExcludedFromBackupKey, kCFBooleanTrue, &mut error) };
    let _error = Owned(error);
    if success == 0 || !excluded(url.0) { return Err(Error::Unavailable); }
    Ok(())
}
