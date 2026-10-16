//! Revision churn is a presentation refresh, never a ready frame or read failure.
use super::{view, StylesFeature};
use serde_json::{json, Value};

impl StylesFeature {
    pub(super) async fn read_snapshot(&self) -> Result<Value, &'static str> {
        for attempt in 0..4 {
            match self.read_snapshot_once().await {
                Err("styles_snapshot_changed" | "account_session_changed" | "styles_session_changed") => {
                    if attempt == 3 {
                        return self.checking_snapshot().await;
                    }
                }
                Ok(view) => {
                    *self.last_snapshot.lock().unwrap() = Some(view.clone());
                    return Ok(view);
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("bounded snapshot attempt returns")
    }

    async fn checking_snapshot(&self) -> Result<Value, &'static str> {
        if self.state.lock().unwrap().stopped {
            return Err("styles_stopped");
        }
        let previous = self.last_snapshot.lock().unwrap().clone();
        let value = match previous {
            Some(view) => view,
            None => {
                // At startup no account frame has agreed yet. Only durable local
                // choices can be shown; this placeholder cannot authorize an action.
                let library = self.library.execute(json!({"kind":"snapshot"}), None, false).await?;
                view::initial_checking(library)?
            }
        };
        if self.state.lock().unwrap().stopped {
            return Err("styles_stopped");
        }
        Ok(view::checking(value))
    }
}
