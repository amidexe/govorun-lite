use crate::{AccountSession, Error, HttpResponse, PromotionEffect};
use serde_json::{json, Value};

pub(super) async fn dispatch(core: &AccountSession, params: Value) -> Result<Value, Error> {
    let epoch = || {
        params["sessionGeneration"]
            .as_u64()
            .ok_or(Error::InvalidRequest)
    };
    let field = |name: &str| params[name].as_str().ok_or(Error::InvalidRequest);
    match params["operation"].as_str() {
        Some("stylePassCrypto") => {
            let captured = epoch()?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            let command = serde_json::from_value(params["command"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = tokio::task::spawn_blocking(move || {
                crate::style_pass_crypto_wire::execute(command)
            })
            .await
            .map_err(|_| Error::InvalidResponse)?
            .map_err(Error::Crypto)?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            Ok(result)
        }
        Some("reconnectRecovery") => {
            core.current(epoch()?)?;
            let facts = serde_json::from_value(params["facts"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            Ok(json!({"action":crate::reconnect_recovery::action(facts)}))
        }
        Some("creditLease") => super::credit_lease(core, &params),
        Some("creditRecoveryWire" | "creditRecoveryRequest") => {
            let snapshot = core.snapshot();
            if snapshot.session_generation != epoch()? {
                return Err(Error::SessionChanged);
            }
            let result = if params["operation"] == "creditRecoveryRequest" {
                crate::credit_recovery_wire::request(
                    serde_json::from_value(params["passes"].clone())
                        .map_err(|_| Error::InvalidRequest)?,
                )?
            } else {
                let status = params["status"]
                    .as_u64()
                    .map(|n| u16::try_from(n).map_err(|_| Error::InvalidRequest))
                    .transpose()?;
                serde_json::to_value(crate::credit_recovery_wire::decide(status, field("body")?))
                    .map_err(|_| Error::InvalidResponse)?
            };
            if core.snapshot().session_generation != snapshot.session_generation {
                return Err(Error::SessionChanged);
            }
            Ok(result)
        }
        Some("creditRecovery") => {
            let snapshot = core.snapshot();
            if snapshot.session_generation != epoch()? {
                return Err(Error::SessionChanged);
            }
            let state = serde_json::from_value(params["state"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let now = params["nowMillis"].as_u64().ok_or(Error::InvalidRequest)?;
            let allowed =
                snapshot.signed_in && snapshot.storage_available && !snapshot.logout_pending;
            let decision = crate::credit_recovery::transition(
                state,
                event,
                snapshot.session_generation,
                now,
                allowed,
            )?;
            if core.snapshot().session_generation != snapshot.session_generation {
                return Err(Error::SessionChanged);
            }
            serde_json::to_value(decision).map_err(|_| Error::InvalidResponse)
        }
        Some("reserveMaintenance") => {
            let snapshot = core.snapshot();
            if snapshot.session_generation != epoch()? {
                return Err(Error::SessionChanged);
            }
            let state = serde_json::from_value(params["state"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let now = params["nowMillis"].as_u64().ok_or(Error::InvalidRequest)?;
            let allowed =
                snapshot.signed_in && snapshot.storage_available && !snapshot.logout_pending;
            let decision = crate::reserve_maintenance::transition(
                state,
                event,
                snapshot.session_generation,
                now,
                allowed,
            )?;
            if core.snapshot().session_generation != snapshot.session_generation {
                return Err(Error::SessionChanged);
            }
            serde_json::to_value(decision).map_err(|_| Error::InvalidResponse)
        }
        Some("sameStyleAccessScope") => {
            core.current(epoch()?)?;
            let first = params
                .get("first")
                .filter(|p| p.is_object())
                .ok_or(Error::InvalidRequest)?;
            let second = params
                .get("second")
                .filter(|p| p.is_object())
                .ok_or(Error::InvalidRequest)?;
            Ok(json!({"same":crate::style_reserve::same_access(first, second)}))
        }
        Some("styleReserveTransition") => {
            let current = core.native_projection();
            if current["snapshot"]["sessionGeneration"].as_u64() != Some(epoch()?)
                || params["revision"].as_u64() != current["snapshot"]["revision"].as_u64()
            {
                return Err(Error::SessionChanged);
            }
            let previous = params
                .get("previous")
                .filter(|value| !value.is_null())
                .cloned();
            crate::style_reserve::transition(previous.map(crate::style_reserve::Projection::decode).transpose()?,
                crate::style_reserve::Projection::decode(current)?)
        }
        Some("styleJobPolling") => {
            let captured = epoch()?;
            core.current(captured)?;
            let state = serde_json::from_value(params["state"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = crate::style_poll::transition(state, event)?;
            core.current(captured)?;
            Ok(result)
        }
        Some("styleJobResponse") => {
            let captured = epoch()?;
            core.current(captured)?;
            let phase = serde_json::from_value(params["phase"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result =
                serde_json::to_value(crate::style_job_response::decide(phase, field("body")?))
                    .map_err(|_| Error::InvalidResponse)?;
            core.current(captured)?;
            Ok(result)
        }
        Some("styleStartAdmission") => {
            let captured = epoch()?;
            core.current(captured)?;
            let state = serde_json::from_value(params["state"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = crate::style_start::transition(state, event)?;
            core.current(captured)?;
            Ok(result)
        }
        Some("styleTextAdmission") => {
            let captured = epoch()?;
            core.current(captured)?;
            let units = serde_json::from_value(params["units"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = crate::style_text::admit(units)?;
            core.current(captured)?;
            Ok(result)
        }
        Some("creditRetirementWire" | "styleReserveBinding") => {
            let captured = epoch()?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            let result = if params["operation"] == "creditRetirementWire" {
                let status = params["status"]
                    .as_u64()
                    .map(|s| u16::try_from(s).map_err(|_| Error::InvalidRequest))
                    .transpose()?;
                let count = params["expectedCount"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or(Error::InvalidRequest)?;
                crate::credit_retirement_wire::decide(
                    status,
                    field("body")?,
                    count,
                    params["retryAfter"].as_str(),
                    params["nowMillis"].as_i64().unwrap_or(0),
                )?
            } else {
                crate::style_reserve_binding::decide(
                    serde_json::from_value(params["facts"].clone())
                        .map_err(|_| Error::InvalidRequest)?,
                )
            };
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            Ok(result)
        }
        Some("styleInventory") => {
            let captured = epoch()?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            let state = serde_json::from_value(params["state"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let now = params["nowMillis"].as_u64().ok_or(Error::InvalidRequest)?;
            let retirement = params["retirementAvailable"]
                .as_bool()
                .ok_or(Error::InvalidRequest)?;
            let result = crate::style_inventory::transition(state, event, now, retirement)?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            serde_json::to_value(result).map_err(|_| Error::InvalidResponse)
        }
        Some("styleIssuanceJournal") => {
            let captured = epoch()?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = crate::style_issuance_journal::evaluate(event)?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            Ok(result)
        }
        Some("prepareStyleIssuance") => {
            let captured = epoch()?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            let preparation = serde_json::from_value(params["preparation"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = crate::style_issuance::prepare(preparation)?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            serde_json::to_value(result).map_err(|_| Error::InvalidResponse)
        }
        Some("styleUsageLedger") => {
            let captured = epoch()?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            let ledger = serde_json::from_value(params["ledger"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let event = serde_json::from_value(params["event"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            let result = crate::usage_ledger::apply(ledger, event)?;
            if core.snapshot().session_generation != captured {
                return Err(Error::SessionChanged);
            }
            serde_json::to_value(result).map_err(|_| Error::InvalidResponse)
        }
        Some("styleWindowPresentation" | "stylePreparation" | "styleReservePlan") => {
            let snapshot = core.snapshot();
            if snapshot.session_generation != epoch()?
                || params["revision"].as_u64() != Some(snapshot.revision)
            {
                return Err(Error::SessionChanged);
            }
            let mut facts: crate::style_window::StyleWindowFacts =
                serde_json::from_value(params["facts"].clone())
                    .map_err(|_| Error::InvalidRequest)?;
            facts.signed_in = snapshot.signed_in;
            facts.profile_fresh = snapshot.profile_fresh;
            facts.entitlement_active = core.style_authority().entitlement_active;
            facts.processing_ready = core.style_authority().processing_ready;
            facts.free_access_available = snapshot.access.free_access_available;
            let profile = core.profile();
            if core.snapshot().revision != snapshot.revision {
                return Err(Error::SessionChanged);
            }
            crate::style_window::bind_profile(
                &mut facts,
                profile.as_ref().map_or(&Value::Null, |p| p.wire()),
            )?;
            if params["operation"] == "styleReservePlan" {
                let mut context: crate::style_refill::Context =
                    serde_json::from_value(params["context"].clone())
                        .map_err(|_| Error::InvalidRequest)?;
                crate::style_refill::bind_profile(
                    &mut context,
                    profile.as_ref().map_or(&Value::Null, |p| p.wire()),
                )?;
                serde_json::to_value(crate::style_refill::plan(facts, context)?)
                    .map_err(|_| Error::InvalidResponse)
            } else if params["operation"] == "stylePreparation" {
                let mut context: crate::style_preparation::Context =
                    serde_json::from_value(params["context"].clone())
                        .map_err(|_| Error::InvalidRequest)?;
                context.profile_fresh = snapshot.profile_fresh;
                serde_json::to_value(crate::style_preparation::plan(facts, context)?)
                    .map_err(|_| Error::InvalidResponse)
            } else {
                serde_json::to_value(crate::style_window::project(facts)?)
                    .map_err(|_| Error::InvalidResponse)
            }
        }
        Some("accessInteraction") => {
            let snapshot = core.snapshot();
            if snapshot.session_generation != epoch()? {
                return Err(Error::SessionChanged);
            }
            let facts = crate::access_interaction::AccessInteractionFacts::published(
                snapshot.signed_in,
                snapshot.storage_available,
                &snapshot.access,
            );
            let plan = crate::access_interaction::plan(
                facts,
                crate::access_interaction::Action::parse(field("action")?)?,
                params["localTerms"] == true,
                params["confirmed"] == true,
                params["pending"] == true,
            )?;
            serde_json::to_value(plan).map_err(|_| Error::InvalidResponse)
        }
        Some("activityDelay") => Ok(json!({"milliseconds":
            u64::try_from(core.activity_refresh_delay().as_millis()).unwrap_or(u64::MAX)})),
        Some("refreshForActivity") => {
            match core.refresh_for_activity(std::time::Duration::ZERO).await {
                Ok(_) | Err(Error::SignedOut | Error::SessionChanged) => {
                    Ok(core.native_projection())
                }
                Err(error) => Err(error),
            }
        }
        Some("storageRecoveryDelay") => {
            let attempt = params["attempt"].as_u64().ok_or(Error::InvalidRequest)?;
            let delay = match attempt {
                0 => Some(0),
                1 => Some(1_000),
                2 => Some(3_000),
                3 => Some(10_000),
                _ => None,
            };
            Ok(json!({"milliseconds":delay}))
        }
        Some("confirmPromotion") => match core
            .confirm_promotion(epoch()?, field("previewId")?, params["confirmed"] == true)
            .await?
        {
            PromotionEffect::Checkout {
                offer_id,
                preview_id,
            } => Ok(json!({"kind":"checkout", "offerId":offer_id, "previewId":preview_id})),
            PromotionEffect::Redeemed(access) => Ok(json!({"kind":"redeemed", "access":access})),
            PromotionEffect::Rejected(response) => Ok(http(response)),
            PromotionEffect::None => Err(Error::InvalidResponse),
        },
        Some("refreshPromotionAccess") => {
            let access = core
                .inner
                .promotions
                .refresh_redeemed_access(core, epoch()?)
                .await?;
            Ok(json!({"access":access}))
        }
        Some("prepareCheckout") => {
            let channel = match params["channel"].as_str() {
                Some("direct") => crate::CatalogChannel::Direct,
                _ => return Err(Error::InvalidRequest),
            };
            let displayed = params
                .get("displayedCatalog")
                .filter(|value| !value.is_null());
            Ok(http(
                core.prepare_checkout_for_catalog(
                    epoch()?,
                    params["request"].clone(),
                    displayed,
                    channel,
                )
                .await?,
            ))
        }
        Some("confirmCheckout") => {
            if params["confirmed"] != true {
                return Err(Error::ConsentUnconfirmed);
            }
            Ok(http(
                core.confirm_checkout(epoch()?, field("path")?, params["request"].clone())
                    .await?,
            ))
        }
        Some("beginCheckout") => {
            if params["userInitiated"] != true {
                return Err(Error::ConsentUnconfirmed);
            }
            let channel = match params["channel"].as_str() {
                Some("direct") => crate::CatalogChannel::Direct,
                _ => return Err(Error::InvalidRequest),
            };
            let displayed = params
                .get("displayedCatalog")
                .filter(|value| !value.is_null());
            let (kind, response) = match core
                .begin_checkout_interaction(epoch()?, params["request"].clone(), displayed, channel)
                .await?
            {
                crate::checkout::CheckoutInteraction::Confirmation(response) => {
                    ("confirmation", response)
                }
                crate::checkout::CheckoutInteraction::Outcome(response) => ("outcome", response),
            };
            Ok(json!({"kind":kind, "response":checked_http(response)?}))
        }
        Some("confirmPreparedCheckout") => {
            if params["confirmed"] != true {
                return Err(Error::ConsentUnconfirmed);
            }
            checked_http(core.confirm_prepared_checkout(epoch()?, params["request"].clone()).await?)
        }
        Some("reconcileCheckout") => Ok(http(
            core.reconcile_checkout(epoch()?, field("sessionId")?)
                .await?,
        )),
        Some("cancelCheckoutBinding") => {
            if params["confirmed"] != true {
                return Err(Error::ConsentUnconfirmed);
            }
            Ok(http(
                core.cancel_checkout_binding(epoch()?, field("sessionId")?)
                    .await?,
            ))
        }
        _ => Err(Error::InvalidRequest),
    }
}

fn http(response: HttpResponse) -> Value {
    json!({"status":response.status, "body":response.body, "etag":response.etag,
        "retryAfterSeconds":response.retry_after_seconds})
}

// The retired native interaction adapter rejected malformed status metadata.
// Keep that protocol boundary in the common owner for every caller.
fn checked_http(response: HttpResponse) -> Result<Value, Error> {
    if !(100..600).contains(&response.status) { return Err(Error::InvalidResponse); }
    Ok(http(response))
}
