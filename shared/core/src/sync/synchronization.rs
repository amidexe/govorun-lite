use crate::{
    statistics_sync::{Day, Total},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Mode {
    Evaluating,
    Pending,
    Enabled,
    LocalOnly,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Scope {
    account: String,
    generation: i64,
}
impl Scope {
    fn validate(&self) -> Result<()> {
        crate::statistics_ledger_model::account_key(&self.account)?;
        if self.generation < 0 {
            return Err(Error::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Stage {
    Content,
    Preferences,
    Statistics,
}
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Flow {
    scope: Scope,
    stage: Stage,
    failed: bool,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalStatistics {
    total: Total,
    days: Vec<Day>,
}
impl LocalStatistics {
    fn meaningful(&self) -> Result<bool> {
        self.total.validate()?;
        if self.days.len() > 35 {
            return Err(Error::InvalidRequest);
        }
        let mut active = 0;
        for day in &self.days {
            crate::statistics_sync::date(&day.date)?;
            Total {
                words: day.words,
                speech_seconds: day.speech_seconds,
                recognitions: day.recognitions,
            }
            .validate()?;
            if day.words > 0 || day.speech_seconds > 0 || day.recognitions > 0 {
                active += 1;
            }
        }
        Ok(self.total.words >= 1000
            || self.total.speech_seconds >= 600
            || self.total.recognitions >= 100
            || active >= 7)
    }
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Command {
    Enrollment {
        stored: Option<Mode>,
        content: crate::content_sync::Document,
        statistics: LocalStatistics,
        history: bool,
        other_account_history: bool,
        requested: bool,
        remote_data: Option<bool>,
    },
    Start {
        scope: Scope,
    },
    Advance {
        flow: Flow,
        scope: Option<Scope>,
        stage: Stage,
        succeeded: bool,
    },
    Schedule {
        failures: u32,
        succeeded: bool,
        pending: bool,
        jitter: f64,
    },
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    match command {
        Command::Enrollment {
            stored,
            content,
            statistics,
            history,
            other_account_history,
            requested,
            remote_data,
        } => {
            let content = content.meaningful()?;
            let meaningful = statistics.meaningful()? || content;
            let mode = match stored {
                Some(mode @ (Mode::Enabled | Mode::LocalOnly)) => mode,
                _ if requested => Mode::Pending,
                _ if history => Mode::Enabled,
                _ if other_account_history => Mode::Pending,
                _ if remote_data == Some(false) || !meaningful => Mode::Enabled,
                _ => Mode::Pending,
            };
            Ok(json!({"mode":mode}))
        }
        Command::Start { scope } => {
            scope.validate()?;
            let flow = Flow {
                scope,
                stage: Stage::Content,
                failed: false,
            };
            Ok(json!({"action":flow.stage,"flow":flow}))
        }
        Command::Advance {
            mut flow,
            scope,
            stage,
            succeeded,
        } => {
            flow.scope.validate()?;
            if let Some(scope) = &scope {
                scope.validate()?;
            }
            if scope.as_ref() != Some(&flow.scope) {
                return Ok(json!({"action":"cancelled","flow":null}));
            }
            if stage != flow.stage {
                return Err(Error::InvalidRequest);
            }
            flow.failed |= !succeeded;
            flow.stage = match stage {
                Stage::Content => Stage::Preferences,
                Stage::Preferences => Stage::Statistics,
                Stage::Statistics => {
                    return Ok(json!({"action":"complete","flow":null,"succeeded":!flow.failed}))
                }
            };
            Ok(json!({"action":flow.stage,"flow":flow}))
        }
        Command::Schedule {
            failures,
            succeeded,
            pending,
            jitter,
        } => {
            if !jitter.is_finite() || !(0.0..=1.0).contains(&jitter) {
                return Err(Error::InvalidRequest);
            }
            if !pending {
                return Ok(json!({"failures":0,"delayMillis":null}));
            }
            if succeeded {
                return Ok(json!({"failures":0,"delayMillis":800}));
            }
            // A durable rejection must not keep a background loop alive forever.
            // Data stays pending; a later explicit/data/network event may try again.
            let failures = failures.saturating_add(1).min(8);
            if failures == 8 {
                return Ok(json!({"failures":failures,"delayMillis":null}));
            }
            let ceiling = (5000_u64 << (failures - 1).min(6)).min(300_000);
            let floor = ceiling / 2;
            let delay = floor + ((ceiling - floor) as f64 * jitter) as u64;
            Ok(json!({"failures":failures,"delayMillis":delay}))
        }
    }
}
