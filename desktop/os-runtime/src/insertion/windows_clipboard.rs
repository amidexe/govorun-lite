//! Eager opaque Win32 clipboard copies. Each allocated SDK object has one
//! owner; ownership transfers only after SetClipboardData succeeds.
use std::{cell::RefCell, ptr, thread, time::Duration};
use windows_sys::core::w;
use windows_sys::Win32::{
    Foundation::{GetLastError, GlobalFree, SetLastError, HANDLE, HWND},
    Graphics::Gdi::{CopyEnhMetaFileW, DeleteEnhMetaFile, DeleteObject},
    System::{DataExchange::*, Memory::*},
    UI::WindowsAndMessaging::{
        CopyImage, CreateWindowExW, DestroyWindow, IMAGE_BITMAP, LR_CREATEDIBSECTION,
    },
};
type Result<T> = std::result::Result<T, &'static str>;

// A distinct owner for this transaction. A user copy from the host's webview
// must count as a newer writer, even when it belongs to the same process.
struct Window(HWND);
impl Window {
    fn new() -> Result<Self> {
        let window = unsafe {
            CreateWindowExW(
                0,
                w!("STATIC"),
                w!("Govorun clipboard"),
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
            Err("clipboard_owner_unavailable")
        } else {
            Ok(Self(window))
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

struct Open;
impl Open {
    fn retry(owner: HWND) -> Result<Self> {
        for attempt in 0..5 {
            if let Ok(open) = Self::new(owner) {
                return Ok(open);
            }
            if attempt != 4 {
                thread::sleep(Duration::from_millis(25 * (attempt + 1)));
            }
        }
        Err("clipboard_unavailable")
    }
    fn new(owner: HWND) -> Result<Self> {
        if unsafe { OpenClipboard(owner) } == 0 {
            return Err("clipboard_unavailable");
        }
        Ok(Self)
    }
}
impl Drop for Open {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

enum Entry {
    Memory(u32, Vec<u8>),
    Bitmap(u32, HANDLE),
    Enhanced(u32, HANDLE),
}
impl Drop for Entry {
    fn drop(&mut self) {
        unsafe {
            match self {
                Self::Bitmap(_, handle) => {
                    DeleteObject(*handle);
                }
                Self::Enhanced(_, handle) => {
                    DeleteEnhMetaFile(*handle);
                }
                _ => {}
            }
        }
    }
}

struct Copy {
    format: u32,
    handle: HANDLE,
    kind: u8,
}
impl Drop for Copy {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                match self.kind {
                    1 => {
                        DeleteObject(self.handle);
                    }
                    2 => {
                        DeleteEnhMetaFile(self.handle);
                    }
                    _ => {
                        GlobalFree(self.handle);
                    }
                }
            }
        }
    }
}

fn memory(bytes: &[u8]) -> Result<HANDLE> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    if handle.is_null() {
        return Err("clipboard_allocation_failed");
    }
    let address = unsafe { GlobalLock(handle) };
    if address.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err("clipboard_allocation_inaccessible");
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address.cast(), bytes.len());
        GlobalUnlock(handle);
    }
    Ok(handle)
}

fn capture_entries(owner: HWND) -> Result<(u32, Vec<Entry>)> {
    let _open = Open::retry(owner)?;
    let revision = unsafe { GetClipboardSequenceNumber() };
    // OLE's live-object metadata is not transferable clipboard content. Raw
    // copies would restore pointers to the previous provider, not its data.
    let ole = unsafe { [RegisterClipboardFormatW(w!("DataObject")), RegisterClipboardFormatW(w!("Ole Private Data"))] };
    let mut result = Vec::new();
    let mut copied_bytes = 0usize;
    let mut format = 0;
    loop {
        unsafe {
            SetLastError(0);
            format = EnumClipboardFormats(format);
        }
        if format == 0 {
            if unsafe { GetLastError() } != 0 {
                return Err("clipboard_enumeration_failed");
            }
            break;
        }
        if ole.contains(&format) { continue; }
        let source = unsafe { GetClipboardData(format) };
        // Handy's default paste does not require a readable old clipboard.
        // Preserve transferable representations; unavailable ones cannot block text.
        if source.is_null() { continue; }
        let entry = match format {
            2 | 0x82 => {
                let copy = unsafe { CopyImage(source, IMAGE_BITMAP, 0, 0, LR_CREATEDIBSECTION) };
                if copy.is_null() {
                    continue;
                }
                Entry::Bitmap(format, copy)
            }
            14 | 0x8e => {
                let copy = unsafe { CopyEnhMetaFileW(source, ptr::null()) };
                if copy.is_null() {
                    continue;
                }
                Entry::Enhanced(format, copy)
            }
            // Owner/private GDI handles and classic metafile containers require
            // specialised ownership. Leave them out of the optional backup.
            3 | 9 | 0x80 | 0x83 | 0x200..=0x3ff => {
                continue
            }
            _ => {
                let size = unsafe { GlobalSize(source) };
                if size == 0 || copied_bytes.saturating_add(size) > super::snapshot::LIMIT {
                    continue;
                }
                let address = unsafe { GlobalLock(source) };
                if address.is_null() {
                    continue;
                }
                copied_bytes += size;
                let bytes =
                    unsafe { std::slice::from_raw_parts(address.cast::<u8>(), size).to_vec() };
                unsafe {
                    GlobalUnlock(source);
                }
                Entry::Memory(format, bytes)
            }
        };
        result.push(entry);
    }
    if unsafe { GetClipboardSequenceNumber() } != revision {
        return Err("clipboard_changed_capture");
    }
    Ok((revision, result))
}

fn prepare(entries: &[Entry]) -> Result<Vec<Copy>> {
    entries
        .iter()
        .map(|entry| {
            let (format, handle, kind) = match entry {
                Entry::Memory(format, bytes) => (*format, memory(bytes)?, 0),
                Entry::Bitmap(format, source) => (
                    *format,
                    unsafe { CopyImage(*source, IMAGE_BITMAP, 0, 0, LR_CREATEDIBSECTION) },
                    1,
                ),
                Entry::Enhanced(format, source) => (
                    *format,
                    unsafe { CopyEnhMetaFileW(*source, ptr::null()) },
                    2,
                ),
            };
            if handle.is_null() {
                return Err("clipboard_copy_failed");
            }
            Ok(Copy {
                format,
                handle,
                kind,
            })
        })
        .collect()
}

fn apply(copies: &mut [Copy]) -> bool {
    for copy in copies {
        if unsafe { SetClipboardData(copy.format, copy.handle) }.is_null() {
            return false;
        }
        copy.handle = ptr::null_mut();
    }
    true
}

fn write_copies(
    owner: HWND,
    expected: u32,
    copies: &mut [Copy],
    recovery: &mut [Copy],
) -> Result<u32> {
    let _open = Open::retry(owner)?;
    if unsafe { GetClipboardSequenceNumber() } != expected {
        return Err("clipboard_changed");
    }
    if unsafe { EmptyClipboard() } == 0 {
        return Err("clipboard_clear_failed");
    }
    if !apply(copies) {
        // Every recovery allocation exists BEFORE EmptyClipboard. Restore while
        // holding the same lock; no lazy provider or concurrent writer intervenes.
        if unsafe { EmptyClipboard() } != 0 && apply(recovery) {
            return Err("clipboard_write_failed");
        }
        return Err("clipboard_recovery_failed");
    }
    // Close finalises the format set (including OS-synthesised text formats).
    // Read the revision afterwards; the lock-held revision is not final.
    drop(_open);
    // No fallible reopen after mutation: a busy clipboard would otherwise
    // leave our temporary text without a restoration lease. The next physical
    // send and restoration each check this revision again under their guard.
    let revision = unsafe { GetClipboardSequenceNumber() };
    if unsafe { GetClipboardOwner() } != owner {
        return Err("clipboard_changed_finalisation");
    }
    Ok(revision)
}

pub struct Snapshot {
    original: u32,
    entries: Vec<Entry>,
    restoration: RefCell<Option<(Vec<Copy>, Vec<Copy>)>>,
    owner: Window,
}
pub fn capture(_owner: HWND) -> Result<Snapshot> {
    let owner = Window::new()?;
    let (original, entries) = capture_entries(owner.0)?;
    Ok(Snapshot {
        original,
        entries,
        restoration: RefCell::new(None),
        owner,
    })
}
pub fn set_text(_owner: HWND, saved: &Snapshot, text: &str) -> Result<u32> {
    let bytes: Vec<u8> = text
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_ne_bytes)
        .collect();
    let mut replacement = vec![Copy {
        format: 13,
        handle: memory(&bytes)?,
        kind: 0,
    }];
    let mut recovery = prepare(&saved.entries)?;
    let restoration = prepare(&saved.entries)?;
    let backup = prepare(&saved.entries)?;
    let revision = write_copies(
        saved.owner.0,
        saved.original,
        &mut replacement,
        &mut recovery,
    )?;
    *saved.restoration.borrow_mut() = Some((restoration, backup));
    Ok(revision)
}
pub fn restore(_owner: HWND, saved: &Snapshot, expected: u32) -> Result<bool> {
    let _open = Open::retry(saved.owner.0)?;
    if unsafe { GetClipboardSequenceNumber() } != expected {
        return Ok(false);
    }
    let (mut restoration, mut backup) = saved
        .restoration
        .borrow_mut()
        .take()
        .ok_or("clipboard_restore_unavailable")?;
    // Keep the same clipboard lock across the CAS and write; write_copies
    // reopens would fail, so apply the already prepared handles directly.
    if unsafe { EmptyClipboard() } == 0 {
        return Err("clipboard_restore_failed");
    }
    if !apply(&mut restoration) {
        if unsafe { EmptyClipboard() } == 0 || !apply(&mut backup) {
            return Err("clipboard_restore_failed");
        }
    }
    Ok(true)
}
