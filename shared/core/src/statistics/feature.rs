//! Statistics commands share one durable ledger and one public view.
use crate::features::{
    dictionary::{Configure, Events},
    dto::dto,
    snapshot_gate::SnapshotGate,
};
use chrono::{Days, NaiveDate};
use govorun_core::{
    statistics_repository::{StatisticsContext, StatisticsDocumentError, StatisticsRepository},
    DocumentStore,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;

pub use govorun_core::sync_enrollment::DataSyncMode as SyncMode;
dto!(StatisticsTotal {
    words: u64,
    seconds: u64,
    sessions: u64
});
dto!(StatisticsDay {
    date: String,
    words: u64,
    seconds: u64,
    sessions: u64
});
dto!(StatisticsSnapshot { today: String, days: Vec<StatisticsDay>, total: StatisticsTotal,
    signed_in: bool, sync_mode: SyncMode, has_cloud_snapshot: bool, storage_recovered: bool,
    storage_read_only: bool });
dto!(StatisticsReset { confirmed: bool });

pub struct ViewContext {
    pub today: String,
    /// The selected statistics account, not merely a restored login identity.
    pub account: Option<String>,
    pub signed_in: bool,
    pub sync_mode: SyncMode,
}
pub type Context = Arc<dyn Fn() -> Result<ViewContext, &'static str> + Send + Sync>;

pub struct StatisticsFeature {
    repository: StatisticsRepository,
    context: Context,
    configure: Configure,
    events: Events,
    gate: SnapshotGate,
    serial: Mutex<()>,
    pending_local: AtomicBool,
}
impl StatisticsFeature {
    pub fn new(
        store: Arc<dyn DocumentStore>,
        context: Context,
        configure: Configure,
        events: Events,
    ) -> Arc<Self> {
        Arc::new(Self {
            repository: StatisticsRepository::new(store),
            context,
            configure,
            events,
            gate: SnapshotGate::default(),
            serial: Mutex::new(()),
            pending_local: AtomicBool::new(false),
        })
    }

    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let confirmed = match method {
            "statistics.snapshot" if params.as_object().is_some_and(|value| value.is_empty()) => {
                false
            }
            "statistics.reset" => {
                let input: StatisticsReset =
                    serde_json::from_value(params).map_err(|_| "statistics_invalid_request")?;
                if !input.confirmed {
                    return Err("statistics_invalid_request");
                }
                true
            }
            _ => return Err("statistics_invalid_request"),
        };
        let event = if confirmed {
            json!({"kind":"reset"})
        } else {
            json!({"kind":"inspect"})
        };
        Ok(self.execute(event, confirmed, confirmed).await?["snapshot"].clone())
    }

    /// Private synchronization callers receive ledger receipts, never UI events
    /// containing account identifiers or the durable journal.
    pub async fn execute(
        &self,
        mut event: Value,
        confirmed_reset: bool,
        local_change: bool,
    ) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let context = (self.context)()?;
        if event["kind"] == "reset" && event.get("account").is_none() {
            event["account"] = json!(context.account);
        }
        let result = self
            .repository
            .execute(
                event,
                StatisticsContext {
                    today: context.today.clone(),
                    account: context.account.clone(),
                },
                confirmed_reset,
            )
            .await
            .map_err(code)?;
        if local_change && result.changed {
            self.pending_local.store(true, Ordering::SeqCst);
        }
        let ledger_projection = result.result["projection"].clone();
        let read_only = !result.writable;
        let configuration = json!({"result":result.result,"revision":result.revision,
            "changed":result.changed,"writable":result.writable,
            "localChange":self.pending_local.load(Ordering::SeqCst)});
        (self.configure)(configuration.clone()).await?;
        self.pending_local.store(false, Ordering::SeqCst);
        // A login/logout can finish while storage or configuration awaits.
        // Publish with the current view context, never the earlier identity.
        let snapshot = projection(&ledger_projection, &(self.context)()?, read_only)?;
        if self.gate.changed(&snapshot) {
            (self.events)("statistics.changed", snapshot.clone());
        }
        Ok(json!({"configuration":configuration,"snapshot":snapshot}))
    }
}

fn projection(
    projection: &Value,
    context: &ViewContext,
    read_only: bool,
) -> Result<Value, &'static str> {
    let invalid = "statistics_invalid_response";
    let today = NaiveDate::parse_from_str(&context.today, "%Y-%m-%d").map_err(|_| invalid)?;
    let view = context
        .account
        .as_ref()
        .and_then(|id| projection["accounts"].get(id))
        .unwrap_or(&projection["local"]);
    let total = totals(&view["total"])?;
    let values = view["days"].as_array().ok_or(invalid)?;
    let mut days = Vec::with_capacity(14);
    // Match the macOS presentation, including zero days, on both platforms.
    for offset in (0..14).rev() {
        let date = today
            .checked_sub_days(Days::new(offset))
            .ok_or(invalid)?
            .to_string();
        let value = values
            .iter()
            .find(|day| day["date"].as_str() == Some(&date));
        let total = value.map(totals).transpose()?.unwrap_or(StatisticsTotal {
            words: 0,
            seconds: 0,
            sessions: 0,
        });
        days.push(StatisticsDay {
            date,
            words: total.words,
            seconds: total.seconds,
            sessions: total.sessions,
        });
    }
    serde_json::to_value(StatisticsSnapshot {
        today: context.today.clone(),
        days,
        total,
        signed_in: context.signed_in,
        sync_mode: context.sync_mode,
        has_cloud_snapshot: view["hasCloud"].as_bool().ok_or(invalid)?,
        storage_recovered: projection["storageRecovered"].as_bool().ok_or(invalid)?,
        storage_read_only: read_only,
    })
    .map_err(|_| invalid)
}
fn totals(value: &Value) -> Result<StatisticsTotal, &'static str> {
    let count = |key| {
        value[key]
            .as_u64()
            .filter(|value| *value <= 9_007_199_254_740_991)
            .ok_or("statistics_invalid_response")
    };
    Ok(StatisticsTotal {
        words: count("words")?,
        seconds: count("speechSeconds")?,
        sessions: count("recognitions")?,
    })
}
fn code(error: StatisticsDocumentError) -> &'static str {
    match error {
        StatisticsDocumentError::InvalidAction => "statistics_invalid_request",
        StatisticsDocumentError::InvalidStorage => "statistics_storage_invalid",
        StatisticsDocumentError::StorageUnavailable => "statistics_storage_unavailable",
        StatisticsDocumentError::Conflict => "statistics_conflict",
    }
}
