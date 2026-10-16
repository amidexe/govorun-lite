//! Opaque process-local UI scopes; no account/device identity or persistent state.
use std::sync::{atomic::{AtomicU64, Ordering}, OnceLock};

pub fn nonce() -> u64 {
    static NONCE: OnceLock<u64> = OnceLock::new();
    *NONCE.get_or_init(|| {
        let mut bytes = [0; 8];
        if getrandom::fill(&mut bytes).is_ok() { return u64::from_le_bytes(bytes); }
        // Scope is never an authorization or crypto key. The serial still prevents
        // reuse within a lifetime if an OS entropy provider is unavailable.
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |time| time.as_nanos() as u64)
    })
}
pub fn next() -> String {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    format!("{:x}-{:x}", nonce(), SERIAL.fetch_add(1, Ordering::Relaxed))
}
