/// One foreground processing policy. Native ports supply monotonic clocks and
/// cancellation; receiving a response never renews the overall deadline.
pub(crate) const ADMISSION_MILLIS: u64 = 7_000;
pub(crate) const START_ATTEMPT_MILLIS: u64 = 4_000;
pub(crate) const STATUS_REQUEST_MILLIS: u64 = 3_000;
/// Говорун.Эхо's note (request class "note"): nobody waits at a cursor, so it
/// keeps the server's 120 s window plus admission and delivery.
pub(crate) const NOTE_MILLIS: u64 = 130_000;

pub(crate) fn deadline(characters: usize) -> u64 {
    match characters {
        0..=2_000 => 10_000,
        2_001..=4_000 => 13_000,
        4_001..=6_000 => 18_000,
        6_001..=8_000 => 23_000,
        _ => 27_000,
    }
}
