//! Caller-provided microphone and compute preferences, without SDK types.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Preferences {
    pub preferred: String,
    pub channel: Option<usize>,
    pub warm: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Compute {
    Automatic,
    Cpu,
    Gpu,
}
