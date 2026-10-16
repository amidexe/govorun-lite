//! Private dictation-adapter requests. This is not a renderer route; raw text
//! and paid capabilities return only to the requesting adapter's reply pipe.
use super::Stylization;
use govorun_core::{stylization::foreground::Stage, Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "operation",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    #[serde(rename = "stylizationProcess")]
    Process {
        session_generation: u64,
        payload: Value,
        preparation_millis: u64,
        #[serde(default)]
        request_id: Option<String>,
    },
    #[serde(rename = "stylizationBeginRecording")]
    BeginRecording { session_generation: u64 },
    #[serde(rename = "stylizationEndRecording")]
    EndRecording { request_id: String },
    #[serde(rename = "stylizationPrepare")]
    Prepare {
        session_generation: u64,
        explicit_demand: bool,
        background_allowed: bool,
        required_credits: usize,
    },
    #[serde(rename = "stylizationPreparationPlan")]
    PreparationPlan {
        session_generation: u64,
        phase: String,
        required_credits: usize,
    },
    #[serde(rename = "stylizationJoinPreparation")]
    JoinPreparation {
        session_generation: u64,
        required_credits: usize,
    },
    #[serde(rename = "stylizationSettingsChanged")]
    SettingsChanged {
        session_generation: u64,
        invalidate_scope: bool,
    },
    #[serde(rename = "stylizationObservation")]
    Observation {},
    #[serde(rename = "stylizationRecover")]
    Recover { session_generation: u64 },
    #[serde(rename = "stylizationOutcome")]
    Outcome {
        session_generation: u64,
        pass: String,
        outcome: String,
        latency_bucket: String,
    },
    #[serde(rename = "stylizationFailure")]
    Failure {
        session_generation: u64,
        reason: String,
    },
    #[serde(rename = "stylizationConnectivity")]
    Connectivity {
        session_generation: u64,
        online: bool,
    },
}
impl Command {
    pub fn decode(value: Value) -> Result<Self> {
        serde_json::from_value(value).map_err(|_| Error::InvalidRequest)
    }
    pub fn recognizes(operation: &str) -> bool {
        matches!(
            operation,
            "stylizationProcess"
                | "stylizationBeginRecording"
                | "stylizationEndRecording"
                | "stylizationPrepare"
                | "stylizationObservation"
                | "stylizationSettingsChanged"
                | "stylizationPreparationPlan"
                | "stylizationJoinPreparation"
                | "stylizationRecover"
                | "stylizationOutcome"
                | "stylizationFailure"
                | "stylizationConnectivity"
        )
    }
}
impl Stylization {
    pub(crate) async fn dispatch_configured(
        &self,
        command: Command,
        selected: bool,
        online: bool,
        progress: &(dyn Fn(Stage) + Send + Sync),
    ) -> Result<Value> {
        if let Command::BeginRecording { session_generation } = command {
            if !selected || !online || self.foreground.allowed(session_generation).is_err() {
                self.foreground.close_connection(None);
                return Ok(json!({"requestId":null}));
            }
            self.foreground.availability().recording_started()?;
            let request_id = random_id()?;
            self.foreground.prepare_connection(request_id.clone());
            return Ok(json!({"requestId":request_id}));
        }
        if matches!(command, Command::Process { .. } | Command::Prepare { .. }) && !selected {
            return Err(Error::ConsentUnconfirmed);
        }
        if let Command::JoinPreparation {
            session_generation,
            required_credits,
        } = command
        {
            let elapsed = self
                .join_preparation(session_generation, required_credits, selected, online)
                .await?;
            Ok(json!({"elapsedMilliseconds":elapsed}))
        } else if let Command::PreparationPlan {
            session_generation,
            phase,
            required_credits,
        } = command
        {
            self.current(session_generation)?;
            if phase == "start" && selected && online {
                self.foreground.availability().recording_started()?;
            }
            let view = self.observation(false, false).await?;
            self.current(session_generation)?;
            view.preparation(&phase, selected, online, required_credits)
        } else {
            self.dispatch(command, progress).await
        }
    }
    pub async fn dispatch(
        &self,
        command: Command,
        progress: &(dyn Fn(Stage) + Send + Sync),
    ) -> Result<Value> {
        match command {
            Command::Process {
                session_generation,
                payload,
                preparation_millis,
                request_id,
            } => {
                let request_id = request_id.map(Ok).unwrap_or_else(random_id)?;
                let _connection = ConnectionGuard(&self.foreground, &request_id);
                let result = self
                    .process(
                        session_generation,
                        payload,
                        random_id()?,
                        request_id.clone(),
                        preparation_millis,
                        progress,
                    )
                    .await;
                let result = result?;
                Ok(
                    json!({"text":result.text,"outcomePass":result.outcome_pass,"latencyBucket":result.latency_bucket}),
                )
            }
            Command::Prepare {
                session_generation,
                explicit_demand,
                background_allowed,
                required_credits,
            } => {
                let result = self
                    .prepare(
                        session_generation,
                        explicit_demand,
                        background_allowed,
                        required_credits,
                        random_id()?,
                    )
                    .await?;
                Ok(
                    json!({"ready":result.ready,"retryAfterMillis":result.retry_after_millis,"blockedReason":result.blocked_reason}),
                )
            }
            // The composition root supplies authoritative selection and physical network facts.
            Command::BeginRecording { .. }
            | Command::PreparationPlan { .. }
            | Command::JoinPreparation { .. } => Err(Error::InvalidRequest),
            Command::EndRecording { request_id } => {
                if !crate::request::uuid(&request_id) {
                    return Err(Error::InvalidRequest);
                }
                self.foreground.close_connection(Some(&request_id));
                Ok(json!({}))
            }
            Command::SettingsChanged {
                session_generation,
                invalidate_scope,
            } => {
                self.current(session_generation)?;
                if invalidate_scope {
                    self.vault
                        .change(
                            govorun_core::style_pass_vault::Action::Inventory(
                                govorun_core::style_inventory::Event::ChangeScope {
                                    compatible: false,
                                },
                            ),
                            (self.clock)(),
                            true,
                            false,
                            &|| self.current(session_generation).is_ok(),
                        )
                        .await?;
                }
                self.maintain(
                    session_generation,
                    govorun_core::stylization::scheduling::Trigger::Maintenance,
                    true,
                )?;
                Ok(json!({}))
            }
            Command::Observation {} => {
                let result = self.observation(false, false).await?;
                Ok(
                    json!({"window":result.window,"refill":result.refill,"ready":result.ready,
                    "pending":result.pending,"issuancePending":result.issuance_pending,
                    "paused":self.foreground.availability().paused()?}),
                )
            }
            Command::Recover { session_generation } => self.recovery_step(session_generation).await,
            Command::Outcome {
                session_generation,
                pass,
                outcome,
                latency_bucket,
            } => {
                self.current(session_generation)?;
                // This callback follows insertion (or its failure) on both hosts.
                self.vault.checkpoint_usage().await?;
                if let Some(worker) = self.usage_worker.get() {
                    worker.request();
                }
                self.foreground
                    .reporting()
                    .outcome(session_generation, pass, &outcome, &latency_bucket)
                    .await?;
                Ok(json!({}))
            }
            Command::Failure {
                session_generation,
                reason,
            } => {
                self.current(session_generation)?;
                self.foreground.reporting().record(&reason)?;
                Ok(json!({}))
            }
            Command::Connectivity {
                session_generation,
                online,
            } => {
                self.current(session_generation)?;
                if !online {
                    self.foreground.close_connection(None);
                }
                self.connectivity_changed(online)?;
                Ok(json!({}))
            }
        }
    }
}
pub use govorun_core::stylization::random_request_id as random_id;

struct ConnectionGuard<'a>(
    &'a govorun_core::stylization::foreground::Foreground,
    &'a str,
);
impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        self.0.close_connection(Some(self.1));
    }
}
