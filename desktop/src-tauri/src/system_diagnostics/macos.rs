use govorun_core::operational_trace::{FailureReason, OperationalTrace};

// POSIX errors from starting the macOS process. These are not Windows status codes.
pub fn worker_start_failed(at_millis: i64, code: Option<i32>) -> OperationalTrace {
    let reason = match code {
        Some(1 | 13) => FailureReason::Access,
        Some(2) => FailureReason::FileMissing,
        Some(12) => FailureReason::Memory,
        Some(8) => FailureReason::LibraryInvalid,
        _ => FailureReason::StartFailed,
    };
    OperationalTrace::worker_start_failed(at_millis, code, reason)
}

pub fn worker_exit(at_millis: i64, code: Option<i32>) -> OperationalTrace {
    OperationalTrace::worker_exit(at_millis, code, FailureReason::Disconnected)
}
