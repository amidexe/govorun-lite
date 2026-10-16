use super::{PromotionAccess, PromotionAccessKind, PromotionHost, PromotionService};
use crate::{AccountSession, Error, HttpResponse, Result};
use serde_json::{json, Value};
use std::sync::Mutex;

/// Host-only effects. Preview capability IDs never enter the renderer snapshot.
pub enum PromotionEffect {
    None,
    Redeemed(PromotionAccess),
    Checkout {
        offer_id: String,
        preview_id: String,
    },
    Rejected(HttpResponse),
}

#[derive(Default)]
struct Presentation {
    scope: String,
    epoch: u64,
    revision: u64,
    busy: bool,
    confirming: bool,
    preview: Option<Value>,
    message: Option<&'static str>,
    access: Option<PromotionAccess>,
}

/// One window workflow for macOS and Windows: admission, explicit confirmation,
/// cancellation of late previews, error copy and sanitized presentation.
#[derive(Default)]
pub struct PromotionController {
    state: Mutex<Presentation>,
}

/// Dropping a window future must release its own spinner without touching a newer action.
struct Operation<'a> {
    owner: &'a PromotionController,
    epoch: u64,
    revision: u64,
}
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        let mut state = self
            .owner
            .state
            .lock()
            .expect("promotion presentation lock");
        if state.epoch == self.epoch && state.revision == self.revision {
            state.busy = false;
            state.confirming = false;
        }
    }
}

impl PromotionController {
    pub fn handles(action: &str) -> bool {
        matches!(
            action,
            "previewPromo"
                | "confirmPromo"
                | "dismissPromo"
                | "refreshPromoAccess"
                | "dismissPromoAccess"
        )
    }

    pub fn decorate(&self, epoch: u64, snapshot: &mut Value) {
        let Some(scope) = snapshot["scope"].as_str() else {
            return;
        };
        let mut state = self.state.lock().expect("promotion presentation lock");
        Self::scope(&mut state, epoch, scope);
        snapshot["promo"] = state.preview.as_ref().map(|preview| json!({
            "id": format!("promotion:{}:{}", state.epoch, state.revision),
            "title": preview["offer"]["confirmationTitle"], "body": preview["offer"]["confirmationBody"],
            "checkoutRequired": preview["offer"]["checkoutRequired"],
            "bonusDays": preview["offer"]["bonus"]["validDays"],
        })).unwrap_or(Value::Null);
        snapshot["promoAccess"] = state
            .access
            .as_ref()
            .map(|access| {
                let mut wire = serde_json::to_value(access).expect("closed access projection");
                wire["id"] = json!(format!(
                    "promotion-access:{}:{}",
                    state.epoch, state.revision
                ));
                wire
            })
            .unwrap_or(Value::Null);
        if state.busy {
            snapshot["busy"] = json!(true);
        }
        if let Some(message) = state.message {
            snapshot["message"] = json!(message);
        }
    }

    pub fn busy(&self) -> bool {
        self.state.lock().expect("promotion presentation lock").busy
    }

    pub fn clear_message(&self) {
        self.state
            .lock()
            .expect("promotion presentation lock")
            .message = None;
    }

    pub async fn execute(
        &self,
        core: &AccountSession,
        epoch: u64,
        scope: &str,
        params: &Value,
    ) -> Result<PromotionEffect> {
        self.execute_with(&core.inner.promotions, core, epoch, scope, params)
            .await
    }

    pub async fn execute_with(
        &self,
        service: &PromotionService,
        host: &dyn PromotionHost,
        epoch: u64,
        scope: &str,
        params: &Value,
    ) -> Result<PromotionEffect> {
        let action = params["action"].as_str().ok_or(Error::InvalidRequest)?;
        if !Self::handles(action) {
            return Err(Error::InvalidRequest);
        }
        if matches!(action, "refreshPromoAccess" | "dismissPromoAccess") {
            return self
                .access_action(service, host, epoch, scope, params)
                .await;
        }
        host.current(epoch)?;
        let (revision, preview_id) = {
            let mut state = self.state.lock().expect("promotion presentation lock");
            Self::scope(&mut state, epoch, scope);
            if action == "dismissPromo" {
                if !state.confirming {
                    state.revision += 1;
                    state.preview = None;
                    state.busy = false;
                    state.message = None;
                }
                return Ok(PromotionEffect::None);
            }
            if state.busy {
                return Err(Error::InvalidRequest);
            }
            let preview_id = if action == "confirmPromo" {
                if params["confirmed"] != true
                    || params["previewId"] != format!("promotion:{}:{}", epoch, state.revision)
                {
                    return Err(Error::InvalidRequest);
                }
                Some(
                    state
                        .preview
                        .as_ref()
                        .and_then(|p| p["previewId"].as_str())
                        .ok_or(Error::InvalidRequest)?
                        .to_owned(),
                )
            } else {
                None
            };
            state.revision += 1;
            state.busy = true;
            state.confirming = preview_id.is_some();
            state.message = None;
            if preview_id.is_none() {
                state.preview = None;
                state.access = None;
            }
            (state.revision, preview_id)
        };
        let _operation = Operation {
            owner: self,
            epoch,
            revision,
        };
        let result = if let Some(id) = preview_id {
            service
                .confirm_promotion(host, epoch, &id, true)
                .await
                .map(|effect| (None, effect))
        } else {
            self.preview(service, host, epoch, params).await
        };
        host.current(epoch)?;
        let mut state = self.state.lock().expect("promotion presentation lock");
        if state.scope != scope
            || state.epoch != epoch
            || state.revision != revision
        {
            return Err(Error::SessionChanged);
        }
        state.busy = false;
        state.confirming = false;
        match result {
            Ok((preview, PromotionEffect::Rejected(response))) => {
                state.preview = preview;
                state.message = Some(response_message(&response));
                Ok(PromotionEffect::None)
            }
            Ok((preview, effect)) => {
                state.preview = preview;
                if let PromotionEffect::Redeemed(access) = &effect {
                    state.message = Some(access.message);
                    state.access =
                        (access.kind != PromotionAccessKind::Done).then(|| access.clone());
                }
                Ok(effect)
            }
            Err(error) => {
                state.preview = None;
                state.message = Some(error_message(error));
                Ok(PromotionEffect::None)
            }
        }
    }

    async fn access_action(
        &self,
        service: &PromotionService,
        host: &dyn PromotionHost,
        epoch: u64,
        scope: &str,
        params: &Value,
    ) -> Result<PromotionEffect> {
        host.current(epoch)?;
        let revision = {
            let mut state = self.state.lock().expect("promotion presentation lock");
            Self::scope(&mut state, epoch, scope);
            if state.busy
                || params["continuationId"]
                    != format!("promotion-access:{}:{}", epoch, state.revision)
            {
                return Err(Error::InvalidRequest);
            }
            let access = state.access.as_ref().ok_or(Error::InvalidRequest)?;
            if params["action"] == "dismissPromoAccess" {
                state.message = Some(if params["deferred"] == true {
                    access.deferred_message
                } else {
                    "Промокод применён."
                });
                state.access = None;
                state.revision += 1;
                return Ok(PromotionEffect::None);
            }
            if access.kind != PromotionAccessKind::Refresh {
                return Err(Error::InvalidRequest);
            }
            state.revision += 1;
            state.busy = true;
            state.revision
        };
        let _operation = Operation {
            owner: self,
            epoch,
            revision,
        };
        let result = service.refresh_redeemed_access(host, epoch).await;
        host.current(epoch)?;
        let mut state = self.state.lock().expect("promotion presentation lock");
        if state.epoch != epoch
            || state.scope != scope
            || state.revision != revision
        {
            return Err(Error::SessionChanged);
        }
        state.busy = false;
        let access = result?;
        state.message = Some(access.message);
        state.access = (access.kind != PromotionAccessKind::Done).then(|| access.clone());
        Ok(PromotionEffect::Redeemed(access))
    }

    async fn preview(
        &self,
        service: &PromotionService,
        host: &dyn PromotionHost,
        epoch: u64,
        params: &Value,
    ) -> Result<(Option<Value>, PromotionEffect)> {
        let code = params["code"].as_str().ok_or(Error::InvalidRequest)?;
        let offer = match params.get("offerId") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => Some(value.as_str()),
            _ => return Err(Error::InvalidRequest),
        };
        let response = service.preview_promotion(host, epoch, code, offer).await?;
        if !(200..300).contains(&response.status) {
            return Ok((None, PromotionEffect::Rejected(response)));
        }
        Ok((Some(response.body), PromotionEffect::None))
    }

    fn scope(state: &mut Presentation, epoch: u64, scope: &str) {
        if state.epoch != epoch || state.scope != scope {
            *state = Presentation {
                epoch,
                scope: scope.into(),
                revision: state.revision + 1,
                ..Default::default()
            };
        }
    }
}

fn response_message(response: &HttpResponse) -> &'static str {
    failure_message(Some(response.status), response.body["error"].as_str())
}

pub(crate) fn failure_message(status: Option<u16>, code: Option<&str>) -> &'static str {
    if status.is_none() {
        return error_message(match code {
            Some("account_network_unavailable") => Error::NetworkUnavailable,
            Some("account_session_changed") => Error::SessionChanged,
            Some("account_signed_out") => Error::SignedOut,
            _ => Error::InvalidResponse,
        });
    }
    match (status.unwrap(), code) {
        (400, Some("invalid_plan")) => "Тарифы ещё загружаются. Повторите через несколько секунд",
        (409, Some("promo_state_changed")) => "Условия изменились. Проверьте промокод ещё раз",
        (409, Some("promo_operation_pending")) => {
            "Дождитесь завершения операции с подпиской и проверьте промокод ещё раз"
        }
        _ => error_message(Error::Http(status.unwrap())),
    }
}

fn error_message(error: Error) -> &'static str {
    match error {
        Error::Http(400) => "Проверьте промокод",
        Error::Http(401) | Error::SignedOut | Error::SessionChanged => {
            "Сессия истекла — войдите снова"
        }
        Error::Http(403) => "Промокод сейчас недоступен",
        Error::Http(429) => "Слишком много попыток. Повторите через 15 минут",
        Error::Http(status) if status >= 500 => {
            "Сервис временно недоступен. Повторите проверку позже"
        }
        Error::NetworkUnavailable => "Нет связи с сервером. Код можно повторить позже",
        _ => "Не удалось проверить код. Попробуйте ещё раз",
    }
}
