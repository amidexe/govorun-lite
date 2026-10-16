//! Normalized key and capture events; concrete OS hooks/drivers supply them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Dictation,
    Plain,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Press,
    Release,
    Abort,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub target: Target,
    pub action: Action,
    pub at_nanos: u64,
}
pub enum Audio<J> {
    Ready { id: u64 },
    Stopped { id: u64, job: J },
    Failed { id: u64, code: &'static str },
    FailedNative { id: u64, code: &'static str, native: crate::journal_report::capture_failure::Native },
    Cancelled { id: u64 },
}
