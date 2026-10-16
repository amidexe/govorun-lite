/// One foreground processing policy. Native ports supply monotonic clocks and
/// cancellation; receiving a response never renews the overall deadline.
pub(crate) const ADMISSION_MILLIS: u64 = 7_000;
pub(crate) const START_ATTEMPT_MILLIS: u64 = 4_000;
pub(crate) const STATUS_REQUEST_MILLIS: u64 = 3_000;

pub(crate) fn deadline(characters: usize) -> u64 {
    match characters {
        0..=2_000 => 10_000,
        2_001..=4_000 => 13_000,
        4_001..=6_000 => 18_000,
        6_001..=8_000 => 23_000,
        _ => 27_000,
    }
}
