//! One durable issuance owner. Cryptographic journals precede Accounts I/O;
//! ambiguous transport/success preserves exactly the same idempotent request.
use super::{Action, Envelope, Repository};
use crate::{
    style_inventory::Event,
    style_pass_crypto_wire::{Command, Key},
    AccountSession, Error, Result,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct Issuance {
    pub(super) repository: Arc<Repository>,
    pub(super) account: AccountSession,
    pub(super) clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}
/// Safe accounting outcome; contains neither paid tokens nor journal material.
#[derive(Clone, Copy)]
pub struct Outcome {
    pub issued: usize,
    pub ready: usize,
    pub refresh_account: bool,
    pub rejected_status: Option<u16>,
    pub retry_after_millis: Option<u64>,
    pub retryable: bool,
    pub quota: Option<crate::stylization::AccessFailure>,
}
impl Outcome {
    /// Fresh quota can end a rejected issuance normally; otherwise only the
    /// pre-commit decision authorizes a retry. A generic 429 is terminal.
    pub fn maintenance(self, needed: bool) -> crate::stylization::scheduling::Completion {
        use crate::stylization::scheduling::{Completion, Outcome};
        let outcome = if self.refresh_account && !needed {
            Outcome::Ready
        } else if self.retryable {
            Outcome::Retry
        } else {
            Outcome::Terminal
        };
        Completion {
            outcome,
            retry_after_millis: self
                .retry_after_millis
                .filter(|_| !matches!(outcome, Outcome::Ready)),
            status: self.rejected_status,
            wire_attempted: true,
        }
    }
}
impl Issuance {
    pub fn new(
        repository: Arc<Repository>,
        account: AccountSession,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Self {
        Self {
            repository,
            account,
            clock,
        }
    }
    fn current(&self, epoch: u64) -> bool {
        self.account.current(epoch).is_ok()
    }
    pub(super) fn authorized(&self, epoch: u64) -> Result<()> {
        self.account.current(epoch)?;
        let snapshot = self.account.snapshot();
        if !snapshot.signed_in
            || !snapshot.storage_available
            || snapshot.logout_pending
            || !self.account.style_authority().authorized
            || !self.account.style_access_current((self.clock)())
        {
            return Err(Error::SessionChanged);
        }
        Ok(())
    }
    pub(super) async fn mutate(
        &self,
        epoch: u64,
        action: Action,
    ) -> Result<super::InventoryChange> {
        self.authorized(epoch)?;
        self.repository
            .change(action, (self.clock)(), true, false, &|| self.current(epoch))
            .await
    }
    pub async fn warm(
        &self,
        epoch: u64,
        count: usize,
        access_end: Option<u64>,
        idempotency_key: String,
    ) -> Result<Outcome> {
        if count > crate::style_inventory::BATCH || !crate::request::uuid(&idempotency_key) {
            return Err(Error::InvalidRequest);
        }
        self.authorized(epoch)?;
        let normalized = self
            .mutate(epoch, Action::Inventory(Event::Normalize))
            .await?;
        let mut state = normalized.candidate;
        let scope = state.scope_epoch()?.ok_or(Error::InvalidRequest)?;
        if !state.stored_value()["issuance"].is_null() {
            let replay = state
                .format()
                .date_millis(&state.stored_value()["issuance"]["replayUntil"])?;
            if (self.clock)() >= replay {
                let id = string(
                    &state.stored_value()["issuance"],
                    state.format().journal_id_field(),
                )?;
                let previous_scope = journal_scope(&state)?;
                state = self
                    .mutate(
                        epoch,
                        Action::Inventory(Event::DiscardIssuance {
                            id,
                            scope_epoch: previous_scope,
                            discard_receipts: false,
                        }),
                    )
                    .await?
                    .candidate;
            }
        }
        if state.stored_value()["issuance"].is_null() {
            if count == 0 {
                return Ok(Outcome {
                    issued: 0,
                    ready: normalized.ready,
                    refresh_account: false,
                    rejected_status: None,
                    retry_after_millis: None,
                    retryable: false,
                    quota: None,
                });
            }
            state = self
                .prepare(epoch, count, access_end, idempotency_key, scope)
                .await?;
        }
        let format = state.format();
        let journal = state.stored_value()["issuance"].clone();
        let id = string(&journal, format.journal_id_field())?;
        let key_id = string(&journal, format.key_id_field())?;
        let count = journal["requests"]
            .as_array()
            .ok_or(Error::StorageInvalid)?
            .len();
        let messages = crypto(journal_crypto(&journal, format, None)?).await?;
        self.authorized(epoch)?;
        let receipts = journal["replacementReceipts"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let before = self
            .account
            .profile()
            .ok_or(Error::SignedOut)?
            .wire()
            .clone();
        let mut request = super::account_wire::issue_request(
            id.clone(),
            key_id.clone(),
            messages["blindedMessages"].clone(),
            receipts,
        )?;
        let reports = crate::usage_ledger::apply(
            self.repository.inspect(false).await?.usage()?, crate::usage_ledger::Event::Validate,
        )?.reports;
        self.authorized(epoch)?;
        if !reports.is_empty() {
            request.body.as_mut().ok_or(Error::InvalidRequest)?["usageReports"] =
                serde_json::to_value(reports).map_err(|_| Error::StorageInvalid)?;
        }
        let response = self.account.request_for_session(epoch, request).await?;
        self.authorized(epoch)?;
        if !(200..300).contains(&response.status) {
            let failure = crate::style_issue::failure(
                Some(response.status),
                crate::style_issue::FailureBody::from_value(&response.body),
            )?;
            if failure.discard_journal {
                self.mutate(
                    epoch,
                    Action::Inventory(Event::DiscardIssuance {
                        id,
                        scope_epoch: journal_scope(&state)?,
                        discard_receipts: failure.discard_replacement_receipts,
                    }),
                )
                .await?;
            }
            let view = self
                .mutate(epoch, Action::Inventory(Event::Normalize))
                .await?;
            return Ok(Outcome {
                issued: 0,
                ready: view.ready,
                refresh_account: failure.refresh_account,
                rejected_status: Some(response.status),
                retry_after_millis: response
                    .retry_after_seconds
                    .map(|seconds| seconds.saturating_mul(1000)),
                retryable: !failure.discard_journal || failure.refresh_account,
                quota: if response.status == 429 && matches!(crate::style_issue::FailureBody::from_value(&response.body), crate::style_issue::FailureBody::Quota) {
                    Some(crate::style_issue::quota_limit(&response.body).unwrap_or(crate::stylization::AccessFailure::UsageUnavailable))
                } else { None },
            });
        }
        // A malformed success may already have spent credits on Accounts.
        // Do not delete or regenerate its journal, key or messages.
        let received_key = string(&response.body, "keyId").map_err(|_| Error::InvalidResponse)?;
        let signatures = response.body["blindSignatures"]
            .as_array()
            .ok_or(Error::InvalidResponse)?;
        if received_key != key_id || signatures.len() != count {
            return Err(Error::InvalidResponse);
        }
        let signatures = signatures
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or(Error::InvalidResponse))
            .collect::<Result<Vec<_>>>()?;
        let tokens = crypto(journal_crypto(&journal, format, Some(signatures))?).await?;
        self.authorized(epoch)?;
        let after = self
            .account
            .profile()
            .ok_or(Error::SignedOut)?
            .wire()
            .clone();
        if !crate::style_reserve::same_access(&before["styleAccess"], &after["styleAccess"]) {
            self.mutate(
                epoch,
                Action::Inventory(Event::ChangeScope { compatible: false }),
            )
            .await?;
        }
        let expiry = journal["expiresAt"].clone();
        let expires_with_scope = journal["expiresWithScope"].as_bool().unwrap_or(false);
        let passes = tokens["tokens"]
            .as_array()
            .ok_or(Error::InvalidResponse)?
            .iter()
            .map(|token| {
                let mut pass =
                    json!({"token":token,"expiresAt":expiry,"expiresWithScope":expires_with_scope});
                pass[format.key_id_field()] = json!(key_id);
                if !response.body["usageScopeId"].is_null() {
                    pass["usageScopeId"] = response.body["usageScopeId"].clone();
                }
                pass
            })
            .collect();
        let finished = self
            .mutate(epoch, Action::FinishIssuance { id, passes })
            .await?;
        // Another exact replay may already have committed this journal. The
        // transition fences by id and never merges these capabilities twice.
        Ok(Outcome {
            issued: if finished.merged { count } else { 0 },
            ready: finished.ready,
            refresh_account: false,
            rejected_status: None,
            retry_after_millis: None,
            retryable: false,
            quota: None,
        })
    }
    async fn prepare(
        &self,
        epoch: u64,
        count: usize,
        access_end: Option<u64>,
        id: String,
        scope: u64,
    ) -> Result<Envelope> {
        let response = self
            .account
            .request_for_session(epoch, super::account_wire::key()?)
            .await?;
        self.authorized(epoch)?;
        if !(200..300).contains(&response.status) {
            return Err(Error::StyleIssuance { stage: "key", status: response.status });
        }
        let key = Key {
            key_id: string(&response.body, "keyId")?,
            algorithm: string(&response.body, "algorithm")?,
            public_key: string(&response.body, "publicKey")?,
            not_before_millis: date(&response.body["notBefore"])?,
            not_after_millis: date(&response.body["notAfter"])?,
        };
        let now = (self.clock)();
        let validity = crate::style_issuance::prepare(crate::style_issuance::Preparation {
            now_millis: now,
            key_not_after_millis: key.not_after_millis,
            access_end_millis: access_end,
        })?;
        let format = self.repository.storage_format();
        let android_key = json!({"algorithm":key.algorithm,"publicKey":key.public_key,
            "notBefore":super::format::Format::FlatV4.date_value(key.not_before_millis)?,
            "notAfter":super::format::Format::FlatV4.date_value(key.not_after_millis)?});
        let key_id = key.key_id.clone();
        let command = if format == super::format::Format::AndroidV8 {
            Command::PrepareAndroid {
                key,
                count,
                expires_at_millis: validity.expires_at_millis,
                now_millis: now,
            }
        } else {
            Command::Prepare {
                key,
                count,
                expires_at_millis: validity.expires_at_millis,
                now_millis: now,
            }
        };
        let prepared = crypto(command).await?;
        self.authorized(epoch)?;
        for _ in 0..3 {
            let selected = self
                .mutate(epoch, Action::Inventory(Event::SelectReceipts { count }))
                .await?;
            if selected.candidate.scope_epoch()? != Some(scope) {
                return Err(Error::SessionChanged);
            }
            if !selected.candidate.stored_value()["issuance"].is_null() {
                return Ok(selected.candidate);
            }
            let format = selected.candidate.format();
            let requests = if format == super::format::Format::AndroidV8 {
                prepared["requests"]
                    .as_array()
                    .ok_or(Error::InvalidResponse)?
                    .iter()
                    .map(|r| r["state"].clone())
                    .collect::<Vec<_>>()
            } else {
                let snapshots: Vec<crate::style_pass_crypto::RequestSnapshot> =
                    serde_json::from_value(prepared["requests"].clone())
                        .map_err(|_| Error::InvalidResponse)?;
                snapshots
                    .iter()
                    .map(|r| format.stored_request(r))
                    .collect::<Result<Vec<_>>>()?
            };
            let mut journal = json!({"expiresAt":format.date_value(validity.expires_at_millis)?,"replayUntil":format.date_value(validity.replay_until_millis)?,
                "requests":requests,"expiresWithScope":validity.expires_with_scope,"replacementReceipts":selected.selected});
            journal[format.journal_id_field()] = json!(id);
            journal[format.key_id_field()] = json!(key_id);
            if format == super::format::Format::AndroidV8 {
                for name in ["algorithm", "publicKey", "notBefore", "notAfter"] {
                    journal[name] = android_key[name].clone();
                }
            }
            let saved = self
                .mutate(
                    epoch,
                    Action::BeginIssuance {
                        journal,
                        expected_scope_epoch: scope,
                    },
                )
                .await?;
            if saved.applied || !saved.candidate.stored_value()["issuance"].is_null() {
                return Ok(saved.candidate);
            }
        }
        Err(Error::InvalidResponse)
    }
}
fn journal_scope(state: &Envelope) -> Result<Option<u64>> {
    super::envelope::optional_epoch(if state.format() == super::format::Format::LedgerV5 {
        &state.stored_value()["issuanceScopeEpoch"]
    } else {
        &state.stored_value()["issuance"]["scopeEpoch"]
    })
}
fn date(value: &Value) -> Result<u64> {
    super::format::Format::FlatV4
        .date_millis(value)
        .map_err(|_| Error::InvalidResponse)
}
fn string(value: &Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or(Error::InvalidResponse)
}
async fn crypto(command: Command) -> Result<Value> {
    tokio::task::spawn_blocking(move || {
        crate::style_pass_crypto_wire::execute(command).map_err(Error::Crypto)
    })
    .await
    .map_err(|_| Error::InvalidResponse)?
}

fn journal_crypto(
    journal: &Value,
    format: super::format::Format,
    signatures: Option<Vec<String>>,
) -> Result<Command> {
    let values = journal["requests"]
        .as_array()
        .ok_or(Error::StorageInvalid)?;
    if format == super::format::Format::AndroidV8 {
        let key = Key {
            key_id: string(journal, "keyId")?,
            algorithm: string(journal, "algorithm")?,
            public_key: string(journal, "publicKey")?,
            not_before_millis: date(&journal["notBefore"])?,
            not_after_millis: date(&journal["notAfter"])?,
        };
        let requests = serde_json::from_value(json!(values)).map_err(|_| Error::StorageInvalid)?;
        Ok(match signatures {
            Some(signatures) => Command::FinalizeAndroid {
                key,
                requests,
                signatures,
            },
            None => Command::RestoreAndroid { key, requests },
        })
    } else {
        let requests = values
            .iter()
            .map(|v| format.request(v))
            .collect::<Result<_>>()?;
        Ok(match signatures {
            Some(signatures) => Command::Finalize {
                requests,
                signatures,
            },
            None => Command::Restore { requests },
        })
    }
}
