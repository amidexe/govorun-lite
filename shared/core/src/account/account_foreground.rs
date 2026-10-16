//! Track foreground lifetimes for shutdown without sharing control-plane permits.
use crate::{Error, Result};
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone, Copy, Default)]
struct State {
    closed: bool,
    active: usize,
}
pub(super) struct Foreground(watch::Sender<State>);
pub(super) struct Guard(Arc<Foreground>);
impl Default for Foreground {
    fn default() -> Self {
        Self(watch::channel(State::default()).0)
    }
}
impl Foreground {
    pub(super) fn enter(self: &Arc<Self>) -> Result<Guard> {
        let mut result = Err(Error::SessionChanged);
        self.0.send_if_modified(|state| {
            if state.closed {
                return false;
            }
            let Some(active) = state.active.checked_add(1) else {
                result = Err(Error::InvalidResponse);
                return false;
            };
            state.active = active;
            result = Ok(());
            true
        });
        result.map(|_| Guard(self.clone()))
    }
    pub(super) fn close(&self) {
        self.0.send_modify(|state| state.closed = true);
    }
    pub(super) async fn drain(&self) {
        let mut changes = self.0.subscribe();
        while changes.borrow_and_update().active > 0 {
            if changes.changed().await.is_err() {
                break;
            }
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.0 .0.send_modify(|state| state.active -= 1);
    }
}
