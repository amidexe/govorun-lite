//! Operating-system microphone permission facts, independent of product rules.
use std::sync::atomic::{AtomicBool, Ordering};
pub struct Request(AtomicBool);
impl Request {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    pub fn begin(&self, status: Microphone) -> bool {
        status == Microphone::NotDetermined
            && self
                .0
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }
    pub fn pending(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub fn finish(&self) {
        self.0.store(false, Ordering::Release);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Microphone {
    Authorized,
    NotDetermined,
    Denied,
    Restricted,
    Unknown,
}
pub fn macos(status: isize) -> Microphone {
    match status {
        0 => Microphone::NotDetermined,
        1 => Microphone::Restricted,
        2 => Microphone::Denied,
        3 => Microphone::Authorized,
        _ => Microphone::Unknown,
    }
}
pub fn registry(value: &str) -> Microphone {
    if value.eq_ignore_ascii_case("allow") {
        Microphone::Authorized
    } else if value.eq_ignore_ascii_case("deny") {
        Microphone::Denied
    } else {
        Microphone::Unknown
    }
}
pub fn windows(device: Microphone, applications: Microphone, desktop: Microphone) -> Microphone {
    if device == Microphone::Denied || desktop == Microphone::Denied {
        Microphone::Denied
    } else if desktop == Microphone::Authorized {
        Microphone::Authorized
    } else if applications == Microphone::Denied {
        Microphone::Denied
    } else if device == Microphone::Authorized && applications == Microphone::Authorized {
        Microphone::Authorized
    } else {
        Microphone::Unknown
    }
}
