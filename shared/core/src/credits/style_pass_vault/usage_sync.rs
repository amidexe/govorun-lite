//! Accounts receives cumulative numeric usage only. No text, signed pass,
//! anonymous request/job ID, or signing journal can enter this transport.
use super::{Action, Envelope, Repository};
use crate::{usage_ledger as usage, AccountRequest, AccountSession, Error, Method, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct UsageSync {
    account: AccountSession,
    vault: Arc<Repository>,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    gate: Mutex<()>,
}
impl UsageSync {
    pub fn new(
        account: AccountSession,
        vault: Arc<Repository>,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Self {
        Self {
            account,
            vault,
            clock,
            gate: Mutex::new(()),
        }
    }
    fn current(&self, epoch: u64) -> bool {
        self.account.current(epoch).is_ok()
    }
    fn allowed(&self, epoch: u64) -> Result<()> {
        self.account.current(epoch)?;
        if !self.account.snapshot().authority.authorized {
            return Err(Error::SessionChanged);
        }
        Ok(())
    }
    async fn apply(&self, epoch: u64, event: usage::Event) -> Result<Envelope> {
        self.allowed(epoch)?;
        self.vault
            .change(Action::Usage(event), (self.clock)(), true, false, &|| {
                self.current(epoch)
            })
            .await
            .map(|v| v.candidate)
    }
    pub async fn observe(&self, epoch: u64) -> Result<Envelope> {
        self.allowed(epoch)?;
        let profile = self.account.profile().ok_or(Error::SignedOut)?;
        let balance = &profile.wire()["styleAccess"]["usageBalance"];
        let event = match observe(balance)? {
            Some(event) => event,
            None => usage::Event::Validate,
        };
        self.apply(epoch, event).await
    }
    /// Single coalesced pass; failures retain cumulative debt for a later trigger.
    pub async fn flush(&self, epoch: u64) -> Result<usize> {
        let _gate = self.gate.lock().await;
        let stored = self.observe(epoch).await?;
        self.vault.checkpoint_usage().await?;
        let reports = usage::apply(stored.usage()?, usage::Event::Validate)?.reports;
        let mut acknowledged = 0;
        for report in reports {
            self.allowed(epoch)?;
            let body = serde_json::to_value(&report).map_err(|_| Error::InvalidRequest)?;
            let response = self
                .account
                .request_for_session(
                    epoch,
                    {
                        let wire=crate::account_protocol::request_wire::usage_request(body)?;
                        AccountRequest::new(Method::Put,wire["path"].as_str().ok_or(Error::InvalidRequest)?,Some(wire["body"].clone()))?
                    },
                )
                .await?;
            self.allowed(epoch)?;
            if !(200..300).contains(&response.status) {
                return Err(Error::Http(response.status));
            }
            if response.body["scopeId"] != report.scope_id {
                return Err(Error::InvalidResponse);
            }
            let days = serde_json::from_value(response.body["acknowledged"].clone())
                .map_err(|_| Error::InvalidResponse)?;
            self.apply(epoch, usage::Event::Acknowledge { report, days })
                .await?;
            self.vault.checkpoint_usage().await?;
            acknowledged += 1;
        }
        Ok(acknowledged)
    }
}
fn observe(balance: &Value) -> Result<Option<usage::Event>> {
    if balance.is_null() {
        return Ok(None);
    }
    if !balance.is_object() {
        return Err(Error::InvalidResponse);
    }
    let optional = |name: &str, default: Value| {
        balance
            .get(name)
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(default)
    };
    let scope = serde_json::from_value(json!({"scopeId":balance["scopeId"],"day":balance["day"],
        "settlementScopeIds":optional("settlementScopeIds",json!([])),
        "dailySettlementScopeIds":optional("dailySettlementScopeIds",json!([]))}))
    .map_err(|_| Error::InvalidResponse)?;
    let device_days = serde_json::from_value(optional("deviceDays", json!({})))
        .map_err(|_| Error::InvalidResponse)?;
    let device_requests = serde_json::from_value(optional("deviceRequests", json!({})))
        .map_err(|_| Error::InvalidResponse)?;
    Ok(Some(usage::Event::Observe {
        scope,
        device_days,
        device_requests,
        protected_scopes: vec![],
    }))
}
