//! Physical lifetime of one recording. Preparation never delays capture or delivery.
use crate::account_host::AccountHost;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State { id: Option<String>, closed: bool }
pub(super) struct Prepared {
    account: Arc<AccountHost>,
    state: Arc<Mutex<State>>,
}
impl Prepared {
    pub(super) fn start(account: Arc<AccountHost>) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let target = state.clone();
        let owner = account.clone();
        tokio::spawn(async move {
            if target.lock().expect("recording connection").closed { return; }
            let Some(id) = owner.begin_styles_recording().await.ok().flatten() else { return; };
            let closed = {
                let mut state = target.lock().expect("recording connection");
                if !state.closed { state.id = Some(id.clone()); }
                state.closed
            };
            if closed { owner.end_styles_recording(id).await; }
        });
        Self { account, state }
    }
    pub(super) fn prepared_id(&self) -> Option<String> {
        self.state.lock().expect("recording connection").id.clone()
    }
}
impl Drop for Prepared {
    fn drop(&mut self) {
        let id = {
            let mut state = self.state.lock().expect("recording connection");
            state.closed = true;
            state.id.take()
        };
        if let Some(id) = id {
            let owner = self.account.clone();
            tokio::spawn(async move { owner.end_styles_recording(id).await; });
        }
    }
}
