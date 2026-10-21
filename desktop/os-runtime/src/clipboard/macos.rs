use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
pub(super) fn copy(name: Option<&str>, text: &str) -> Result<(), &'static str> {
    objc2::rc::autoreleasepool(|_| {
        // The system clipboard is generalPasteboard ("Apple CFPasteboard general"). A board looked
        // up by the legacy name "NSGeneralPboard" is a private board: writes succeed, nobody can paste.
        let board = match name {
            Some(name) => NSPasteboard::pasteboardWithName(&NSString::from_str(name)),
            None => NSPasteboard::generalPasteboard(),
        };
        let text = NSString::from_str(text);
        board.clearContents();
        if board.setString_forType(&text, unsafe { NSPasteboardTypeString }) {
            Ok(())
        } else {
            Err("clipboard_write_failed")
        }
    })
}
