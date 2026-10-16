//! Local path notifications, with one latest snapshot and no request/retry policy.
use std::sync::{Arc, Mutex};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub online: bool,
    pub revision: u64,
    pub recovery_revision: u64,
}
pub type Handler = Arc<dyn Fn(Event) + Send + Sync>;
#[derive(Default)]
struct Inner {
    closed: bool,
    status: Option<i32>,
    event: Option<Event>,
    handler: Option<Handler>,
}
#[derive(Default)]
pub struct State(Mutex<Inner>);
impl State {
    pub fn close(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.closed = true;
            state.handler = None;
        }
    }
    pub fn available(&self) -> Option<bool> {
        self.0.lock().ok()?.status.map(|status| status != 2)
    }
    pub fn bind(&self, handler: Handler) -> Result<(), &'static str> {
        let event = {
            let mut state = self
                .0
                .lock()
                .map_err(|_| "network_observation_unavailable")?;
            if state.closed {
                return Err("network_closed");
            }
            if state.handler.is_some() {
                return Err("network_already_composed");
            }
            state.handler = Some(handler.clone());
            state.event
        };
        if let Some(event) = event {
            handler(event);
        }
        Ok(())
    }
    /// Network.framework: 1 satisfied, 2 unsatisfied, 3 requires connection.
    /// Windows adapters use 1/2; unknown observations do not declare offline.
    pub fn update(&self, status: i32) {
        if !(1..=3).contains(&status) {
            return;
        }
        let notification = {
            let Ok(mut state) = self.0.lock() else { return };
            if state.closed || state.status == Some(status) {
                return;
            }
            let recovered = status == 1 && state.status != Some(1);
            let previous = state.event;
            let event = Event {
                online: status != 2,
                revision: previous.map_or(1, |event| event.revision.saturating_add(1)),
                recovery_revision: previous
                    .map_or(0, |event| event.recovery_revision)
                    .saturating_add(u64::from(recovered)),
            };
            state.status = Some(status);
            state.event = Some(event);
            state.handler.clone().map(|handler| (handler, event))
        };
        if let Some((handler, event)) = notification {
            handler(event);
        }
    }
    pub fn wake(&self) {
        let notification = {
            let Ok(mut state) = self.0.lock() else { return };
            if state.closed || state.status == Some(2) {
                return;
            }
            let previous = state.event;
            let event = Event {
                online: true,
                revision: previous.map_or(1, |event| event.revision.saturating_add(1)),
                recovery_revision: previous
                    .map_or(0, |event| event.recovery_revision)
                    .saturating_add(1),
            };
            state.event = Some(event);
            state.handler.clone().map(|handler| (handler, event))
        };
        if let Some((handler, event)) = notification {
            handler(event);
        }
    }
}
