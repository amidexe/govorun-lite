//! The desktop documents in the shared base (`govorun-document-store`, 2.1.110).
pub use govorun_document_store::{base_unavailable, Cell, Changes, Db, Legacy, Notice, Rows, FILE};

/// The documents that live in the base; secrets, the journal and identity stay apart.
pub const DOCUMENTS: [&str; 10] = [
    "dictionary.document",
    "statistics.document",
    "sync.content",
    "sync.preferences",
    "sync.enrollment",
    "styles.document",
    "selfHost.document",
    "clipboard.document",
    "hotkeys.document",
    "recording.document",
];
/// Larger old values were never readable; the row keeps only that fact.
pub const OVERSIZED: usize = 8 * 1_048_576;
/// Any account on this computer: its sync choice was lost with a damaged base.
pub const ANY_ACCOUNT: &str = "0000000000000000000000000000000000000000000000000000000000000000";
