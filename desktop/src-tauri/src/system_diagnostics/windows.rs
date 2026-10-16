use govorun_core::operational_trace::{FailureReason, OperationalTrace};

pub fn worker_start_failed(at_millis: i64, code: Option<i32>) -> OperationalTrace {
    let reason = match code {
        Some(2 | 3) => FailureReason::FileMissing,
        Some(5) => FailureReason::Access,
        Some(8 | 14 | 1455) => FailureReason::Memory,
        Some(126) => FailureReason::LibraryMissing,
        Some(193 | 216) => FailureReason::LibraryInvalid,
        _ => FailureReason::StartFailed,
    };
    OperationalTrace::worker_start_failed(at_millis, code, reason)
}

pub fn worker_exit(at_millis: i64, code: Option<i32>) -> OperationalTrace {
    let reason = match code.map(|value| value as u32) {
        Some(0xC0000135) => FailureReason::LibraryMissing,
        Some(0xC000007B) => FailureReason::LibraryInvalid,
        Some(0xC0000017 | 0xC000009A) => FailureReason::Memory,
        Some(0xC0000022) => FailureReason::Access,
        _ => FailureReason::Disconnected,
    };
    OperationalTrace::worker_exit(at_millis, code, reason)
}
