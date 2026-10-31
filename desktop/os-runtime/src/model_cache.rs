//! Local compute resources; no durable preferences or user data.
//! Jobs hold session handles independently of this cache.
use std::time::{Duration, Instant};
pub use govorun_core::features::dictation::inputs::Compute;
/// The idle time of a settings choice in minutes; 0 keeps the resource loaded.
pub fn idle_timeout(minutes: u32) -> Option<Duration> {
    (minutes != 0).then(|| Duration::from_secs(u64::from(minutes) * 60))
}
pub trait Loader {
    type Key: Clone + PartialEq;
    type Session: Clone;
    fn load(&mut self, key: &Self::Key) -> Result<Self::Session, &'static str>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    Standby,
    Loading,
    Ready,
    Failed,
}
pub struct Cache<L: Loader> {
    loader: L,
    desired: L::Key,
    retained: Option<(L::Key, L::Session)>,
    /// Idle time before the resource is released; `None` keeps it loaded.
    timeout: Option<Duration>,
    idle_since: Option<Instant>,
    failed: bool,
}
impl<L: Loader> Cache<L> {
    pub fn new(loader: L, desired: L::Key, timeout: Option<Duration>) -> Self {
        Self {
            loader,
            desired,
            retained: None,
            timeout,
            idle_since: None,
            failed: false,
        }
    }
    pub fn readiness(&self) -> Readiness {
        if self
            .retained
            .as_ref()
            .is_some_and(|(key, _)| *key == self.desired)
        {
            Readiness::Ready
        } else if self.failed {
            Readiness::Failed
        } else {
            Readiness::Standby
        }
    }
    pub fn session(&mut self) -> Result<L::Session, &'static str> {
        self.idle_since = None;
        if let Some((key, session)) = &self.retained {
            if *key == self.desired {
                return Ok(session.clone());
            }
        }
        let session = match self.loader.load(&self.desired) {
            Ok(session) => session,
            Err(error) => {
                self.failed = true;
                return Err(error);
            }
        };
        self.failed = false;
        self.retained = Some((self.desired.clone(), session.clone()));
        Ok(session)
    }
    /// Busy jobs keep their admitted session; compute changes take effect on
    /// the next admission, never midway through a recognition chunk.
    pub fn configure(
        &mut self,
        desired: L::Key,
        timeout: Option<Duration>,
        busy: bool,
    ) -> Result<(), &'static str> {
        if desired == self.desired {
            if timeout != self.timeout {
                self.idle_since = None;
            }
            self.timeout = timeout;
            return Ok(());
        }
        if busy {
            self.desired = desired;
            self.timeout = timeout;
            self.failed = false;
            self.idle_since = None;
            return Ok(());
        }
        // Load before replacement: failure preserves the usable old resource
        // and desired configuration for durable rollback in the host.
        let session = match &self.retained {
            Some((key, session)) if *key == desired => session.clone(),
            _ => self.loader.load(&desired)?,
        };
        self.retained = Some((desired.clone(), session));
        self.desired = desired;
        self.timeout = timeout;
        self.failed = false;
        self.idle_since = None;
        Ok(())
    }
    /// The silent host timer cannot unload a capture or queued recognition.
    pub fn idle(&mut self, now: Instant, busy: bool) -> bool {
        let Some(timeout) = self.timeout else {
            self.idle_since = None;
            return false;
        };
        if busy {
            self.idle_since = None;
            return false;
        }
        if self.retained.is_none() {
            return false;
        }
        let since = *self.idle_since.get_or_insert(now);
        if now.saturating_duration_since(since) < timeout {
            return false;
        }
        self.retained = None;
        self.failed = false;
        self.idle_since = None;
        true
    }
    pub fn idle_deadline(&self) -> Option<Instant> {
        if self.retained.is_none() {
            return None;
        }
        let timeout = self.timeout?;
        self.idle_since.and_then(|since| since.checked_add(timeout))
    }
}
