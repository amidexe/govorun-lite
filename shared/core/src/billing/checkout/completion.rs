use super::{CheckoutHost, CheckoutService};
use crate::{AccountRequest, Error, Future, HttpResponse, Method, ProfileSource, Result};
use serde_json::json;

struct Source<'a>(&'a dyn CheckoutHost);
impl ProfileSource for Source<'_> {
    fn current(&self, epoch: u64) -> Result<()> { self.0.current(epoch) }
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>> {
        self.0.request(epoch, request)
    }
}

impl CheckoutService {
    /// The caller holds the checkout operation lock and has admitted/persisted
    /// the server result. Applying access never replays the payment request.
    pub(super) async fn complete_response(&self, host: &dyn CheckoutHost, epoch: u64,
        identity: u64, key: &str, mut response: HttpResponse, journaled: bool) -> Result<HttpResponse> {
        if !(200..300).contains(&response.status) {
            return self.present_failure(epoch, identity, response);
        }
        response = super::presentation::response("/v1/billing/tbank/checkout", response)?;
        if !matches!(
            response.body["clientCheckout"]["disposition"].as_str(), Some("connected" | "scheduled")) {
            return Ok(response);
        }
        self.finish_completion(host, epoch, identity, key, response, journaled,
            "Подписка подтверждена. Обновляем данные аккаунта. Повторять оплату не нужно.").await
    }

    pub(super) async fn complete_plan_change(&self, host: &dyn CheckoutHost, epoch: u64,
        identity: u64, key: &str, mut response: HttpResponse, journaled: bool) -> Result<HttpResponse> {
        if !(200..300).contains(&response.status) {
            return self.present_failure(epoch, identity, response);
        }
        response.body["clientCheckout"] = json!({
            "kind": "plan_change", "disposition": "completed", "launchAction": "complete", "canCancel": false,
            "resumeUrl": null, "expiresInSeconds": null,
            // The receipt is historical. Current terms are in the freshly read
            // billing status, which may reflect a later change on another device.
            "message": "Изменение тарифа подтверждено. Актуальные условия показаны в подписке."
        });
        self.finish_completion(host, epoch, identity, key, response, journaled,
            "Изменение тарифа подтверждено. Обновляем данные аккаунта. Повторять изменение не нужно.").await
    }

    async fn finish_completion(&self, host: &dyn CheckoutHost, epoch: u64,
        identity: u64, key: &str, mut response: HttpResponse, journaled: bool,
        pending_message: &str) -> Result<HttpResponse> {
        let already_applied = journaled && self.checked(epoch, identity)?.attempt.as_ref()
            .is_some_and(|attempt| attempt.completed);
        let applied = if already_applied { true } else {
            let result = self.apply_checkout_account(host, epoch, identity, key, journaled).await;
            host.current(epoch)?;
            drop(self.checked(epoch, identity)?);
            match result {
                Ok(()) => true,
                Err(Error::SessionChanged | Error::SignedOut) => return Err(Error::SessionChanged),
                Err(_) => false,
            }
        };
        let projection = &mut response.body["clientCheckout"];
        projection["accessRefreshPending"] = json!(!applied);
        if !applied {
            // The financial result remains in the server fields and journal.
            // Client lifecycle stays pending until access and storage catch up.
            projection["disposition"] = json!("pending");
            projection["launchAction"] = json!("reconcile");
            projection["canCancel"] = json!(false);
            projection["resumeUrl"] = serde_json::Value::Null;
            projection["expiresInSeconds"] = serde_json::Value::Null;
            projection["message"] = json!(pending_message);
        }
        Ok(response)
    }

    async fn apply_checkout_account(&self, host: &dyn CheckoutHost, epoch: u64,
        identity: u64, key: &str, journaled: bool) -> Result<()> {
        let update = host.begin_checkout_profile(epoch)?;
        if update.identity.account_id.as_deref() != Some(host.checkout_account(epoch)?.as_str()) {
            return Err(Error::InvalidResponse);
        }
        let status = host.request(epoch, AccountRequest::new(Method::Get, "/v1/billing/status", None)?).await;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        crate::validate_billing_status(&status?.success()?.body)?;
        let profile = crate::load_account_profile(&Source(host), epoch, update.identity.clone()).await?;
        drop(self.checked(epoch, identity)?);
        host.publish_checkout_profile(epoch, update, profile).await?;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        if journaled {
            self.persist(host, epoch, identity, key, None).await?;
            // Retain the response in memory to deduplicate another confirm from
            // the same form. A new disclosure is a new explicit user intent.
            self.checked(epoch, identity)?.attempt.as_mut().ok_or(Error::InvalidRequest)?.completed = true;
        } else if self.checked(epoch, identity)?.observed.is_some() {
            self.checked(epoch, identity)?.observed = None;
        }
        Ok(())
    }
}
