use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
pub(super) fn copy(name: &str, text: &str) -> Result<(), &'static str> {
    objc2::rc::autoreleasepool(|_| {
        let board = NSPasteboard::pasteboardWithName(&NSString::from_str(name));
        let text = NSString::from_str(text);
        board.clearContents();
        if board.setString_forType(&text, unsafe { NSPasteboardTypeString }) {
            Ok(())
        } else {
            Err("clipboard_write_failed")
        }
    })
}
