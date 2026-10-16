//! Cooperative cancellation with an optional physical abort effect.
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
#[derive(Clone, Default)]
pub struct Cancellation {
    cancelled: Arc<AtomicBool>,
    abort: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl Cancellation {
    pub fn with_abort(abort: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            cancelled: Arc::default(),
            abort: Some(Arc::new(abort)),
        }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(abort) = &self.abort {
            abort();
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
