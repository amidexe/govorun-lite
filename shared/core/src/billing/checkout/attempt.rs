use super::{
    confirmation::{confirmation, new_key},
    journal::{slot, Journal},
    prepare_checkout, CheckoutHost,
};
use crate::{AccountRequest, CatalogChannel, Error, HttpResponse, Method, Result};
use serde_json::Value;
use std::sync::{Mutex, MutexGuard};
use tokio::sync::Mutex as AsyncMutex;

pub(super) struct Attempt {
    pub(super) intent: Value,
    pub(super) request: AccountRequest,
    pub(super) response: Option<HttpResponse>,
    pub(super) dispatched: bool,
    pub(super) completed: bool,
}

impl Attempt {
    pub(super) fn unresolved(&self) -> bool {
        self.response.as_ref().is_none_or(|reply| {
            if self.request.path() == super::plan_change::PATH {
                return (200..300).contains(&reply.status) && !self.completed;
            }
            super::session::disposition(&reply.body).is_ok_and(|kind|
                kind == "pending" || (!self.completed && matches!(kind, "connected" | "scheduled")))
        })
    }
}

#[derive(Default)]
pub(super) struct State {
    pub(super) epoch: u64,
    pub(super) identity: u64,
    pub(super) account: Option<String>,
    pub(super) preview: Option<Value>,
    pub(super) selection: Option<Value>,
    pub(super) attempt: Option<Attempt>,
    pub(super) observed: Option<super::observed::ObservedCheckout>,
    pub(super) revision: u64,
    pub(super) activity: Option<super::lifecycle::Activity>,
    pub(super) discovery: super::discovery::Discovery,
}

/// One protected, account-scoped confirmed attempt. Persist before dispatch;
/// ambiguous outcomes retain the exact body/key across cancellation and restart.
#[derive(Default)]
pub struct CheckoutService {
    pub(super) state: Mutex<State>,
    pub(super) operation: AsyncMutex<()>,
}

impl CheckoutService {
    /// Management and checkout dispatch share one operation boundary. Restore
    /// the protected journal before allowing a command after a process restart.
    pub(crate) async fn management_scope<'a>(&'a self, host: &dyn CheckoutHost,
        epoch: u64) -> Result<tokio::sync::MutexGuard<'a, ()>> {
        host.current(epoch)?;
        let operation = self.operation.try_lock().map_err(|_| Error::InvalidRequest)?;
        let (identity, _) = self.restore(host, epoch).await?;
        let _activity = self.begin_activity(host, epoch, identity, super::lifecycle::Activity::Prepare)?;
        self.discover_before_new_operation(host, epoch, identity).await?;
        let state = self.checked(epoch, identity)?;
        if state.discovery != super::discovery::Discovery::Verified || state.observed.is_some() || state.attempt.as_ref().is_some_and(Attempt::unresolved) {
            return Err(Error::InvalidRequest);
        }
        drop(state);
        Ok(operation)
    }

    pub fn identity_changed(&self, epoch: u64) {
        let mut state = self.state.lock().expect("checkout state lock");
        *state = State {
            epoch,
            identity: state.identity.wrapping_add(1),
            revision: state.revision + 1,
            ..Default::default()
        };
    }

    pub(super) fn scope(&self, epoch: u64) -> u64 {
        let mut state = self.state.lock().expect("checkout state lock");
        if state.epoch != epoch {
            *state = State {
                epoch,
                identity: state.identity.wrapping_add(1),
                revision: state.revision + 1,
                ..Default::default()
            };
        }
        state.identity
    }

    pub(super) fn checked(&self, epoch: u64, identity: u64) -> Result<MutexGuard<'_, State>> {
        let state = self.state.lock().expect("checkout state lock");
        if state.epoch != epoch || state.identity != identity {
            return Err(Error::SessionChanged);
        }
        Ok(state)
    }

    pub(super) async fn restore(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
    ) -> Result<(u64, String)> {
        host.current(epoch)?;
        let identity = self.scope(epoch);
        let account = host.checkout_account(epoch)?;
        let key = slot(&account)?;
        {
            let state = self.checked(epoch, identity)?;
            if let Some(owner) = &state.account {
                return if owner == &account {
                    Ok((identity, key))
                } else {
                    Err(Error::SessionChanged)
                };
            }
        }
        let record = host.load_checkout(epoch, key.clone()).await?;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        let mut state = self.checked(epoch, identity)?;
        if let Some(value) = record {
            let record = Journal::decode(&account, value)?;
            state.attempt = Some(Attempt {
                intent: record.intent()?,
                request: record.request(),
                response: record.response(),
                // A crash between the durable write and HTTP cannot prove non-dispatch.
                dispatched: true,
                completed: false,
            });
            state.preview = Some(record.preview);
            state.selection = Some(record.selection);
        }
        state.account = Some(account);
        state.revision += 1;
        Ok((identity, key))
    }

    pub(super) fn record(
        &self,
        epoch: u64,
        identity: u64,
        reply: Option<&HttpResponse>,
    ) -> Result<Value> {
        let state = self.checked(epoch, identity)?;
        Journal::encode(
            state.account.as_deref().ok_or(Error::StorageInvalid)?,
            state.selection.as_ref().ok_or(Error::StorageInvalid)?,
            state.preview.as_ref().ok_or(Error::StorageInvalid)?,
            &state.attempt.as_ref().ok_or(Error::StorageInvalid)?.request,
            reply,
        )
    }

    pub(super) async fn persist(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
        identity: u64,
        key: &str,
        record: Option<Value>,
    ) -> Result<()> {
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        let result = host.save_checkout(epoch, key.to_owned(), record).await;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        result
    }

    pub async fn prepare(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
        selection: Value,
        displayed: Option<&Value>,
        channel: CatalogChannel,
    ) -> Result<HttpResponse> {
        host.current(epoch)?;
        super::preparation::validate_request(&selection)?;
        let _operation = self.operation.lock().await;
        host.current(epoch)?;
        let identity = self.scope(epoch);
        let _activity = self.begin_activity(host, epoch, identity, super::lifecycle::Activity::Prepare)?;
        self.prepare_locked(host, epoch, selection, displayed, channel).await
    }

    pub(super) async fn prepare_locked(
        &self, host: &dyn CheckoutHost, epoch: u64, selection: Value,
        displayed: Option<&Value>, channel: CatalogChannel,
    ) -> Result<HttpResponse> {
        let (identity, key) = self.restore(host, epoch).await?;
        self.discover_before_new_operation(host, epoch, identity).await?;
        self.require_confirmable(epoch, identity)?;
        let needs_clear = {
            let state = self.checked(epoch, identity)?;
            if state.observed.is_some() { return Err(Error::InvalidRequest); }
            if state.attempt.as_ref().is_some_and(Attempt::unresolved) {
                if state.selection.as_ref() != Some(&selection) {
                    return Err(Error::InvalidRequest);
                }
                let preview = state.preview.clone().ok_or(Error::InvalidRequest)?;
                return Ok(response(super::project_preview(preview)));
            }
            state.attempt.as_ref().is_some_and(|attempt| !attempt.completed)
        };
        if needs_clear {
            self.persist(host, epoch, identity, &key, None).await?;
        }
        {
            let mut state = self.checked(epoch, identity)?;
            state.preview = None;
            state.selection = None;
            state.attempt = None;
        }
        let result = prepare_checkout(host, epoch, selection.clone(), displayed, channel).await;
        host.current(epoch)?;
        let mut state = self.checked(epoch, identity)?;
        // A failed refresh never leaves an old unconfirmed disclosure available.
        let result = result?;
        if (200..300).contains(&result.status) {
            state.preview = Some(result.body.clone());
            state.selection = Some(selection);
        }
        Ok(result)
    }

    pub async fn confirm(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
        path: &str,
        input: Value,
    ) -> Result<HttpResponse> {
        self.confirm_command(host, epoch, Some(path), input).await
    }

    pub async fn confirm_prepared(&self, host: &dyn CheckoutHost, epoch: u64,
        input: Value) -> Result<HttpResponse> {
        self.confirm_command(host, epoch, None, input).await
    }

    async fn confirm_command(&self, host: &dyn CheckoutHost, epoch: u64,
        path: Option<&str>, input: Value) -> Result<HttpResponse> {
        host.current(epoch)?;
        let _operation = self.operation.lock().await;
        host.current(epoch)?;
        let identity = self.scope(epoch);
        let _activity = self.begin_activity(host, epoch, identity, super::lifecycle::Activity::Confirm)?;
        self.confirm_with_presentation_locked(host, epoch, identity, path, input).await
    }

    pub(super) async fn confirm_with_presentation_locked(
        &self, host: &dyn CheckoutHost, epoch: u64, identity: u64,
        path: Option<&str>, input: Value,
    ) -> Result<HttpResponse> {
        let result = self.confirm_locked(host, epoch, path, input).await;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        if result.as_ref().is_ok_and(|reply| (200..300).contains(&reply.status)) {
            self.checked(epoch, identity)?.discovery = super::discovery::Discovery::Verified;
        }
        match result {
            Err(error @ (Error::SignedOut | Error::SessionChanged)) => Err(error),
            Err(error) => Err(Error::Checkout(self.confirmation_failure(epoch, identity, error)?)),
            Ok(response) if !(200..300).contains(&response.status) =>
                self.present_failure(epoch, identity, response),
            result => result,
        }
    }

    async fn confirm_locked(&self, host: &dyn CheckoutHost, epoch: u64,
        path: Option<&str>, input: Value) -> Result<HttpResponse> {
        let (identity, key) = self.restore(host, epoch).await?;
        let (path, input) = match path {
            Some(path) => (path, input),
            None => {
                let state = self.checked(epoch, identity)?;
                super::confirmation::prepared(state.preview.as_ref().ok_or(Error::InvalidRequest)?, &input)?
            }
        };
        // A stored result is not a new purchase. Keep definitive refusals stable
        // across restart; the exact original intent is still checked below.
        if self.checked(epoch, identity)?.attempt.is_none() {
            self.require_confirmable(epoch, identity)?;
        }
        let cached = self.checked(epoch, identity)?.attempt.as_ref().and_then(|attempt| attempt.response.clone());
        let (request, preview, previously_dispatched) = {
            let mut state = self.checked(epoch, identity)?;
            let preview = state.preview.clone().ok_or(Error::InvalidRequest)?;
            let intent = confirmation(&preview, path, &input)?;
            if let Some(attempt) = &state.attempt {
                if attempt.intent != intent || attempt.request.path() != path {
                    return Err(Error::InvalidRequest);
                }
            } else {
                let mut body = intent.clone();
                body["idempotencyKey"] = Value::String(new_key()?);
                state.attempt = Some(Attempt {
                    intent,
                    request: AccountRequest::internal(Method::Post, path, Some(body)),
                    response: None,
                    dispatched: false,
                    completed: false,
                });
            }
            let attempt = state.attempt.as_ref().unwrap();
            (attempt.request.clone(), preview, attempt.dispatched)
        };
        if let Some(response) = cached {
            return if path == "/v1/billing/tbank/checkout" {
                self.complete_response(host, epoch, identity, &key, response, true).await
            } else { self.complete_plan_change(host, epoch, identity, &key, response, true).await };
        }
        if path == super::plan_change::PATH && previously_dispatched {
            // A lost reply cannot authorize replay, even with the same key:
            // creation admission can change after the original mutation.
            let target = self.recovery_target(epoch, identity, key, "latest")?;
            let result = host.request(epoch, AccountRequest::internal(Method::Get, &target.path, None)).await;
            return self.accept_recovery(host, epoch, &target, result).await;
        }
        // Even a failed write may have committed. Keep the exact attempt in memory.
        self.persist(
            host,
            epoch,
            identity,
            &key,
            Some(self.record(epoch, identity, None)?),
        )
        .await?;
        self.checked(epoch, identity)?
            .attempt
            .as_mut()
            .unwrap()
            .dispatched = true;
        let result = host.request(epoch, request).await;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        let result = result?;
        let success = (200..300).contains(&result.status);
        if success {
            super::session::matches_confirmation(&result.body, &preview)?;
            if path == "/v1/billing/tbank/checkout" {
                super::session::validate_recovery(&result.body)?;
            }
        }
        let terminal = (result.status == 424 && result.body["error"] == "checkout_attempt_failed")
            || (success
                && matches!(
                    result.body["state"].as_str(),
                    Some("cancelled" | "failed" | "expired")
                ));
        if terminal {
            self.persist(host, epoch, identity, &key, None).await?;
            // Retain disclosure: only a new explicit confirmation creates another attempt.
            self.checked(epoch, identity)?.attempt = None;
        } else if success || (!previously_dispatched && matches!(result.status, 400 | 409)) {
            // After an ambiguous dispatch, 400/409 cannot prove the previous request
            // never charged. A confirmed reply becomes visible only after it is durable.
            self.persist(
                host,
                epoch,
                identity,
                &key,
                Some(self.record(epoch, identity, Some(&result))?),
            )
            .await?;
            self.checked(epoch, identity)?
                .attempt
                .as_mut()
                .unwrap()
                .response = Some(result.clone());
        }
        if path == "/v1/billing/tbank/checkout" {
            self.complete_response(host, epoch, identity, &key, result, !terminal).await
        } else { self.complete_plan_change(host, epoch, identity, &key, result, !terminal).await }
    }
}

fn response(body: Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        body,
        etag: None,
        retry_after_seconds: None,
    }
}
