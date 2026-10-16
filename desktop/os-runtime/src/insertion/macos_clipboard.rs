//! AppKit representations stay opaque. Lazy reads run only in the short-lived
//! main-thread helper; host writes use copies materialized before mutation.
use super::snapshot::{Snapshot, LIMIT};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};

fn board(name: &str) -> Retained<NSPasteboard> {
    NSPasteboard::pasteboardWithName(&NSString::from_str(name))
}
pub fn revision(name: &str) -> i64 {
    board(name).changeCount() as i64
}
pub fn capture(name: &str) -> Result<Snapshot, &'static str> {
    let board = board(name);
    let revision = board.changeCount() as i64;
    let mut copies = Vec::new();
    let mut skipped = false;
    let mut bytes = 0usize;
    if let Some(items) = board.pasteboardItems() {
        if items.len() > 128 {
            return Err("clipboard_snapshot_too_large");
        }
        for item in items.iter() {
            let mut formats = Vec::new();
            let types = item.types();
            if types.len() > 256 {
                return Err("clipboard_snapshot_too_large");
            }
            for kind in types.iter() {
                if let Some(data) = item.dataForType(&kind) {
                    bytes = bytes.saturating_add(data.length());
                    if bytes > LIMIT {
                        return Err("clipboard_snapshot_too_large");
                    }
                    formats.push((kind.to_string(), data.to_vec()));
                } else {
                    skipped = true;
                }
            }
            if !formats.is_empty() {
                copies.push(formats);
            }
        }
    }
    if copies.is_empty() && board.types().is_some_and(|types| !types.is_empty()) {
        skipped = true;
    }
    if board.changeCount() as i64 != revision {
        return Err("clipboard_changed");
    }
    Snapshot::from_parts(revision, skipped, copies)
}
pub struct Saved {
    snapshot: Snapshot,
    copies: Retained<NSArray<ProtocolObject<dyn NSPasteboardWriting>>>,
}
pub fn prepare(snapshot: Snapshot) -> Result<Saved, &'static str> {
    let mut items = Vec::new();
    for formats in &snapshot.items {
        let item = NSPasteboardItem::new();
        for (kind, bytes) in formats {
            let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
            if !item.setData_forType(&data, &NSString::from_str(kind)) {
                return Err("clipboard_snapshot_invalid");
            }
        }
        items.push(item);
    }
    let references: Vec<_> = items
        .iter()
        .map(|item| ProtocolObject::<dyn NSPasteboardWriting>::from_ref(&**item))
        .collect();
    Ok(Saved {
        snapshot,
        copies: NSArray::from_slice(&references),
    })
}
fn apply(board: &NSPasteboard, saved: &Saved) -> bool {
    board.clearContents();
    saved.copies.is_empty() || board.writeObjects(&saved.copies)
}
pub fn set_text(name: &str, saved: &Saved, text: &str) -> Result<i64, &'static str> {
    let board = board(name);
    let text = NSString::from_str(text);
    if board.changeCount() as i64 != saved.snapshot.revision {
        return Err("clipboard_changed");
    }
    board.clearContents();
    let written = board.setString_forType(&text, unsafe { NSPasteboardTypeString });
    let revision = board.changeCount() as i64;
    if !written {
        if board.changeCount() as i64 == revision && !apply(&board, saved) {
            return Err("clipboard_restore_failed");
        }
        return Err("clipboard_write_failed");
    }
    Ok(revision)
}
pub fn restore(name: &str, saved: &Saved, expected: i64) -> Result<bool, &'static str> {
    let board = board(name);
    if board.changeCount() as i64 != expected {
        return Ok(false);
    }
    if !apply(&board, saved) {
        return Err("clipboard_restore_failed");
    }
    Ok(true)
}
