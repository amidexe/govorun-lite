use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Cancellation { requested: AtomicBool, signal: tokio::sync::Notify }
impl Cancellation {
    pub fn request(&self) { self.requested.store(true, Ordering::SeqCst); self.signal.notify_waiters(); }
    pub fn requested(&self) -> bool { self.requested.load(Ordering::SeqCst) }
    pub async fn wait(&self) {
        loop {
            let notified = self.signal.notified();
            if self.requested() { return; }
            notified.await;
        }
    }
}
