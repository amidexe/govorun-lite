//! Desktop owns the feature lifetime; the common session owns every financial decision.
use super::{dto::BillingSnapshot, format::Dates, links, snapshot};
use crate::features::snapshot_gate::SnapshotGate;
use govorun_core::{AccountSession, PromotionController};
use crate::features::ports::Reply;
use crate::features::runtime::session::Session as HostSession;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::Semaphore;

pub type Events = Arc<dyn Fn(&str, Value) + Send + Sync>;
pub type Opener = Arc<dyn Fn(&str) -> Result<(), &'static str> + Send + Sync>;
#[derive(Default)]
struct State {
    epoch: u64,
    serial: u64,
    operation: Option<u64>,
    presentation: bool,
    message: Option<String>,
}

pub struct BillingHost {
    core: AccountSession,
    stylization: std::sync::OnceLock<Arc<crate::features::stylization::Stylization>>,
    native: HostSession,
    documents: std::sync::OnceLock<Arc<crate::features::documents::DocumentsHost>>,
    terms: u32,
    scheme: String,
    instance: String,
    state: Mutex<State>,
    snapshots: tokio::sync::Mutex<()>,
    gate: SnapshotGate,
    events: Events,
    open: Opener,
    requests: Arc<Semaphore>,
    stopped: AtomicBool,
}

impl BillingHost {
    pub fn new(
        core: AccountSession,
        native: HostSession,
        terms: u32,
        scheme: String,
        events: Events,
        open: Opener,
        requests: Arc<Semaphore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            core,
            stylization: std::sync::OnceLock::new(),
            native,
            documents: std::sync::OnceLock::new(),
            terms,
            scheme,
            instance: crate::instance::next(),
            state: Mutex::new(State::default()),
            snapshots: tokio::sync::Mutex::new(()),
            gate: SnapshotGate::default(),
            events,
            open,
            requests,
            stopped: AtomicBool::new(false),
        })
    }
    pub fn attach_documents(
        &self,
        documents: Arc<crate::features::documents::DocumentsHost>,
    ) -> Result<(), &'static str> {
        self.documents
            .set(documents)
            .map_err(|_| "documents_already_initialized")
    }
    pub fn attach_stylization(
        &self,
        owner: Arc<crate::features::stylization::Stylization>,
    ) -> Result<(), &'static str> {
        self.stylization
            .set(owner)
            .map_err(|_| "stylization_already_initialized")
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("billing host state")
    }
    fn scope(&self, epoch: u64, signed_in: bool) -> String {
        format!("{}:{epoch}:{signed_in}", self.instance)
    }
    fn current(&self, epoch: u64) -> bool {
        !self.stopped.load(Ordering::SeqCst) && self.core.snapshot().session_generation == epoch
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    async fn view(&self) -> Result<BillingSnapshot, &'static str> {
        let projection = self.core.native_projection();
        let account = &projection["snapshot"];
        let epoch = account["sessionGeneration"]
            .as_u64()
            .ok_or("account_invalid_response")?;
        let style = self
            .stylization
            .get()
            .ok_or("stylization_unavailable")?
            .observation(false, false)
            .await
            .map_err(|error| error.code())?
            .window;
        if self.core.snapshot().revision
            != account["revision"]
                .as_u64()
                .ok_or("account_invalid_response")?
        {
            return Err("styles_snapshot_changed");
        }
        let (message, operation) = {
            let mut state = self.state();
            if state.epoch != epoch {
                *state = State {
                    epoch,
                    serial: state.serial + 1,
                    ..State::default()
                };
            }
            (state.message.clone(), state.operation.is_some())
        };
        let mut view = snapshot::snapshot(
            &projection,
            &style,
            self.scope(epoch, account["signedIn"] == true),
            self.terms,
            message,
            Dates::default(),
        )?;
        view.busy |= operation;
        view.status.loading |= operation;
        Ok(view)
    }
    fn decorated(&self, view: BillingSnapshot) -> Reply {
        let mut value = serde_json::to_value(view).map_err(|_| "account_invalid_response")?;
        self.core.decorate_promotions(&mut value);
        Ok(value)
    }
    pub async fn snapshot(&self) -> Reply {
        let _read = self.snapshots.lock().await;
        self.decorated(self.view().await?)
    }
    pub async fn publish(&self) {
        if self.stopped.load(Ordering::SeqCst) {
            return;
        }
        let _read = self.snapshots.lock().await;
        if let Ok(view) = self.view().await.and_then(|view| self.decorated(view)) {
            if self.gate.changed(&view) {
                (self.events)("billing.changed", view);
            }
        }
    }
    pub async fn invoke(self: &Arc<Self>, method: &str, params: Value) -> Reply {
        if self.stopped.load(Ordering::SeqCst) {
            return Err("native_closing");
        }
        if method == "billing.snapshot" {
            return self.snapshot().await;
        }
        if method == "billing.callback" {
            if !params["url"]
                .as_str()
                .is_some_and(|value| links::bank_return(value, &self.scheme))
            {
                return Err("invalid_parameters");
            }
            let epoch = self.core.snapshot().session_generation;
            self.dispatch(epoch, json!({"action":"recover"}), false)
                .await?;
            return Ok(Value::Null);
        }
        if method != "billing.action" {
            return Err("unsupported_command");
        }
        let view = self.view().await?;
        if params["scope"] != view.scope {
            return Err("account_session_changed");
        }
        let action = params["action"]
            .as_str()
            .ok_or("invalid_parameters")?
            .to_owned();
        let epoch = self.core.snapshot().session_generation;
        if !self.current(epoch) || view.scope != self.scope(epoch, self.core.snapshot().signed_in) {
            return Err("account_session_changed");
        }
        if matches!(action.as_str(), "support" | "legal") {
            self.state().message = None;
            let projection = self.core.native_projection();
            let url = if action == "support" {
                links::support(&projection, &view, Dates::default())
            } else {
                links::legal(&projection, &params)?
            };
            self.open(url).await?;
            return self.snapshot().await;
        }
        let background = matches!(action.as_str(), "dismissPreview" | "resumeCheckout");
        let promotion = PromotionController::handles(&action);
        if !promotion
            && !matches!(
                action.as_str(),
                "refresh"
                    | "recover"
                    | "restartBinding"
                    | "previewCheckout"
                    | "choosePaymentMethod"
                    | "confirmCheckout"
                    | "resumeCheckout"
                    | "dismissPreview"
                    | "cancelRenewal"
                    | "cancelPlanChange"
                    | "removeCard"
                    | "requestRefund"
                    | "activateStarter"
            )
        {
            return Err("invalid_parameters");
        }
        if !background
            && !matches!(action.as_str(), "dismissPromo" | "dismissPromoAccess")
            && (view.busy
                || self.core.promotion_busy()
                || promotion && (!view.signed_in || view.status.loading))
        {
            return Err("billing_busy");
        }
        let mut command = params;
        command["userInitiated"] = json!(true);
        if action == "dismissPreview" {
            let mut state = self.state();
            if state.presentation {
                state.operation = None;
                state.serial += 1;
            }
        }
        if promotion {
            self.state().message = None;
            let _permit = self.request_permit(epoch).await?;
            let result = self.subscription_result(epoch, command).await?;
            self.effect(epoch, &result["effect"]).await?;
            self.publish().await;
            return self.snapshot().await;
        }
        self.core.clear_promotion_message();
        self.dispatch(epoch, command, !background).await?;
        self.snapshot().await
    }
    async fn dispatch(
        self: &Arc<Self>,
        epoch: u64,
        command: Value,
        foreground: bool,
    ) -> Result<(), &'static str> {
        let permit = self.request_permit(epoch).await?;
        let serial = if foreground {
            let mut state = self.state();
            if state.operation.is_some() {
                return Err("billing_busy");
            }
            state.epoch = epoch;
            state.serial += 1;
            state.operation = Some(state.serial);
            state.message = None;
            state.presentation = matches!(
                command["action"].as_str(),
                Some("previewCheckout" | "choosePaymentMethod")
            );
            Some(state.serial)
        } else {
            None
        };
        let host = self.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let result = host.execute(epoch, command).await;
            if host.current(epoch) {
                let mut state = host.state();
                if serial.is_none() || state.operation == serial {
                    if serial.is_some() && result.is_err() {
                        state.message = Some(
                            "Не удалось выполнить действие. Проверьте связь и попробуйте ещё раз."
                                .into(),
                        );
                    }
                    if serial.is_some() {
                        state.operation = None;
                        state.presentation = false;
                    }
                }
            }
            host.publish().await;
        });
        self.publish().await;
        Ok(())
    }
    async fn request_permit(&self, epoch: u64) -> Result<tokio::sync::OwnedSemaphorePermit, &'static str> {
        let permit = self.requests.clone().acquire_owned().await.map_err(|_| "native_closing")?;
        if !self.current(epoch) { return Err("account_session_changed"); }
        Ok(permit)
    }
    async fn subscription_result(&self, epoch: u64, command: Value) -> Reply {
        let result = self.core.subscription_action(epoch, command).await
            .map_err(|error| error.code())?;
        self.native.publish_authority(self.core.native_projection()).await?;
        Ok(result)
    }
    async fn execute(&self, epoch: u64, mut command: Value) -> Result<(), &'static str> {
        if command["action"] == "activateStarter" {
            let options = self
                .documents
                .get()
                .ok_or("documents_unavailable")?
                .activation_options(epoch, command["activationOptions"].clone())
                .await?;
            command["activationOptions"] = options;
        }
        if !self.current(epoch) {
            return Err("account_session_changed");
        }
        let result = self.subscription_result(epoch, command).await?;
        self.effect(epoch, &result["effect"]).await
    }
    async fn effect(&self, epoch: u64, effect: &Value) -> Result<(), &'static str> {
        if !self.current(epoch) {
            return Err("account_session_changed");
        }
        match effect["kind"].as_str() {
            None if effect.is_null() => Ok(()),
            Some("openUrl") => {
                self.open(links::https(effect["url"].as_str().ok_or("invalid_parameters")?)?.into())
                    .await
            }
            Some("activateStyle") => {
                self.documents
                    .get()
                    .ok_or("documents_unavailable")?
                    .apply_subscription(epoch, effect)
                    .await
            }
            // The sanitized promotion continuation is rendered by Svelte; it
            // does not represent an additional OS or settings effect.
            Some("promotionAccess") => Ok(()),
            _ => Err("account_invalid_response"),
        }
    }
    async fn open(&self, url: String) -> Result<(), &'static str> {
        let open = self.open.clone();
        tokio::task::spawn_blocking(move || open(&url))
            .await
            .map_err(|_| "link_unavailable")?
    }
}
