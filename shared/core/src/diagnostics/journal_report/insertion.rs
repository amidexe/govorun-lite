//! Closed physical insertion failures; no text or clipboard representations.
pub const CATEGORY: &str = "Вставка";
const CODES: &[&str] = &[
    "insertion_empty", "insertion_unavailable", "insertion_target_changed", "insertion_failed", "insertion_worker_failed", "insertion_permission_required", "clipboard_snapshot_invalid",
    "clipboard_owner_invalid", "clipboard_owner_unavailable", "clipboard_unavailable", "clipboard_enumeration_failed",
    "clipboard_data_unavailable", "clipboard_bitmap_cannot_be_preserved", "clipboard_metafile_cannot_be_preserved",
    "clipboard_format_cannot_be_eagerly_preserved", "clipboard_format_cannot_be_preserved", "clipboard_format_inaccessible",
    "clipboard_allocation_failed", "clipboard_allocation_inaccessible", "clipboard_copy_failed", "clipboard_changed_capture",
    "clipboard_changed", "clipboard_clear_failed", "clipboard_write_failed", "clipboard_recovery_failed",
    "clipboard_changed_finalisation", "clipboard_restore_failed", "clipboard_restore_unavailable", "paste_keys_unavailable",
];
pub fn code(value: &str) -> Option<&'static str> { CODES.iter().copied().find(|code| *code == value) }
pub fn stored(value: &str) -> Option<String> { code(value).map(|code| format!("insertion_failure {code}")) }
pub fn from_stored(value: &str) -> Option<&'static str> { code(value.strip_prefix("insertion_failure ")?) }
pub(crate) fn failure_summary() -> String {
    format!("{} · GV-I01", super::lookup(super::windows_catalog::TITLES,"insertion_failed").unwrap().trim_end_matches('.'))
}
pub(crate) fn failure_detail(code: &str) -> String { format!("Код: GV-I01. Этап: вставка. Причина: {code}.") }
