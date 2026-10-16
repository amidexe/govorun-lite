//! One physical configuration transaction; durable choices belong to the owner.
use super::inputs::{Preferences, Compute};
use std::{future::Future, pin::Pin};
pub type Reply<'a> = Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send + 'a>>;
#[derive(Clone)]
pub struct Config {
    pub input: Preferences,
    pub compute: Compute,
    pub keep_model: bool,
}
pub trait Ports: Send + Sync {
    fn input<'a>(&'a self, input: &'a Preferences) -> Reply<'a>;
    fn model<'a>(&'a self, config: &'a Config) -> Reply<'a>;
}
pub async fn apply(
    ports: &impl Ports,
    previous: &Config,
    next: &Config,
) -> Result<(), &'static str> {
    let input = previous.input != next.input;
    let model = previous.compute != next.compute || previous.keep_model != next.keep_model;
    let changed = async {
        if input {
            ports.input(&next.input).await?;
        }
        if model {
            ports.model(next).await?;
        }
        Ok(())
    }
    .await;
    if let Err(error) = changed {
        // Even a failed ACK may have partially changed the driver. Attempt both
        // restorations; one failure must not prevent recovery of the other.
        let restored_model = if model {
            ports.model(previous).await
        } else {
            Ok(())
        };
        let restored_input = if input {
            ports.input(&previous.input).await
        } else {
            Ok(())
        };
        if restored_model.is_err() || restored_input.is_err() {
            return Err("dictation_configuration_recovery_failed");
        }
        return Err(error);
    }
    Ok(())
}
