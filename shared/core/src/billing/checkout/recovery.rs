use super::{session, CheckoutHost, CheckoutService};
use crate::{AccountRequest, Error, HttpResponse, Method, Result};
use serde_json::{json, Value};

pub(super) struct RecoveryTarget {
    pub(super) identity: u64,
    pub(super) key: String,
    pub(super) path: String,
    pub(super) preview: Option<Value>,
    pub(super) exact_id: Option<String>,
    pub(super) journaled: bool,
    pub(super) plan_change: bool,
    pub(super) observed: bool,
}

impl CheckoutService {
    /// Read the exact journaled or server-observed operation. Never create a purchase or infer absence from /latest.
    pub async fn reconcile(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
        requested: &str,
    ) -> Result<HttpResponse> {
        host.current(epoch)?;
        let _operation = self.operation.lock().await;
        host.current(epoch)?;
        let identity = self.scope(epoch);
        let _activity = self.begin_activity(host, epoch, identity, super::lifecycle::Activity::Reconcile)?;
        let result = self.reconcile_locked(host, epoch, requested).await;
        host.current(epoch)?;
        self.recovery_result(epoch, identity, requested == "latest", false, result)
    }

    pub(super) async fn reconcile_locked(&self, host: &dyn CheckoutHost, epoch: u64,
        requested: &str) -> Result<HttpResponse> {
        if requested != "latest" && !crate::promotions::valid_preview_id(&json!(requested)) {
            return Err(Error::InvalidRequest);
        }
        let (identity, key) = self.restore(host, epoch).await?;
        let target = self.recovery_target(epoch, identity, key, requested)?;
        let result = host
            .request(
                epoch,
                AccountRequest::internal(Method::Get, &target.path, None),
            )
            .await;
        self.accept_recovery(host, epoch, &target, result).await
    }

    pub(super) fn recovery_target(
        &self,
        epoch: u64,
        identity: u64,
        key: String,
        requested: &str,
    ) -> Result<RecoveryTarget> {
        let (path, preview, exact_id, journaled, plan_change) = {
            let state = self.checked(epoch, identity)?;
            if let Some(observed) = &state.observed {
                return Ok(RecoveryTarget { identity, key,
                    path: format!("/v1/billing/tbank/checkout/{}", observed.session_id),
                    preview: None, exact_id: Some(observed.session_id.clone()),
                    journaled: false, plan_change: false, observed: true });
            }
            match &state.attempt {
                Some(attempt) if attempt.request.path() == super::plan_change::PATH
                    && attempt.response.as_ref().is_none_or(|r| (200..300).contains(&r.status)) => {
                    let id = attempt.request.body().and_then(|b| b["idempotencyKey"].as_str())
                        .ok_or(Error::StorageInvalid)?;
                    (format!("/v1/billing/change-attempts/{id}"), state.preview.clone(), None, true, true)
                }
                Some(attempt)
                    if attempt.request.path() == "/v1/billing/tbank/checkout"
                        && attempt
                            .response
                            .as_ref()
                            .is_none_or(|reply| (200..300).contains(&reply.status)) =>
                {
                    let id = attempt
                        .response
                        .as_ref()
                        .and_then(|r| r.body["checkoutSessionId"].as_str())
                        .map(str::to_owned);
                    let path = if let Some(id) = &id {
                        format!("/v1/billing/tbank/checkout/{id}")
                    } else {
                        format!(
                            "/v1/billing/tbank/checkout-attempts/{}",
                            attempt
                                .request
                                .body()
                                .and_then(|b| b["idempotencyKey"].as_str())
                                .ok_or(Error::StorageInvalid)?
                        )
                    };
                    (path, state.preview.clone(), id, true, false)
                }
                _ => (
                    format!("/v1/billing/tbank/checkout/{requested}"),
                    None,
                    (requested != "latest").then(|| requested.to_owned()),
                    false,
                    false,
                ),
            }
        };
        Ok(RecoveryTarget {
            identity,
            key,
            path,
            preview,
            exact_id,
            journaled,
            plan_change,
            observed: false,
        })
    }

    pub(super) async fn accept_recovery(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
        target: &RecoveryTarget,
        result: Result<HttpResponse>,
    ) -> Result<HttpResponse> {
        host.current(epoch)?;
        drop(self.checked(epoch, target.identity)?);
        let mut result = result?;
        if !(200..300).contains(&result.status) {
            if (target.journaled || target.observed) && result.status == 404 {
                // An in-flight creation may not be visible yet. A missing
                // exact operation does not permit another payment key.
                result.status = 503;
                result.body = json!({"error":"checkout_attempt_unresolved"});
            }
            return Ok(result);
        }
        let disposition = if target.plan_change {
            super::plan_change::validate(&result.body,
                target.preview.as_ref().ok_or(Error::StorageInvalid)?, true)?;
            "completed"
        } else { session::validate_recovery(&result.body)? };
        if target
            .exact_id
            .as_ref()
            .is_some_and(|id| result.body["checkoutSessionId"] != *id)
        {
            return Err(Error::InvalidResponse);
        }
        if let Some(preview) = &target.preview {
            session::matches_confirmation(&result.body, preview)?;
        }
        if target.observed && disposition == "terminal" {
            self.checked(epoch, target.identity)?.observed = None;
        } else if !target.journaled && disposition != "terminal" {
            // A definitive refusal may remain durable for repeat confirmation.
            // A distinct server operation replaces that completed local attempt.
            if self.checked(epoch, target.identity)?.attempt.is_some() {
                self.persist(host, epoch, target.identity, &target.key, None).await?;
            }
            self.retain_observed_checkout(epoch, target.identity, &result.body)?;
        } else if target.journaled {
            if disposition == "terminal" {
                self.persist(host, epoch, target.identity, &target.key, None)
                    .await?;
                let mut state = self.checked(epoch, target.identity)?;
                state.attempt = None;
                state.preview = None;
                state.selection = None;
            } else {
                let already_applied = {
                    let mut state = self.checked(epoch, target.identity)?;
                    let attempt = state.attempt.as_mut().ok_or(Error::InvalidRequest)?;
                    if attempt.response.as_ref().is_none_or(|old| old.body["state"] != result.body["state"]) {
                        attempt.completed = false;
                    }
                    attempt.completed
                };
                if !already_applied {
                    self.persist(host, epoch, target.identity, &target.key,
                        Some(self.record(epoch, target.identity, Some(&result))?)).await?;
                }
                self.checked(epoch, target.identity)?
                    .attempt
                    .as_mut()
                    .unwrap()
                    .response = Some(result.clone());
            }
        }
        if target.plan_change {
            self.complete_plan_change(host, epoch, target.identity, &target.key, result, target.journaled).await
        } else {
            self.complete_response(host, epoch, target.identity, &target.key, result, target.journaled).await
        }
    }
}
