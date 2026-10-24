//! Drain the composed owners on failed startup and explicit shutdown.
use super::*;

impl Services {
    pub(super) async fn finish_startup(&self, result: Reply) -> Reply {
        // A partially composed session must never become a cached READY
        // snapshot on the next request. Explicit shutdown keeps its flag.
        if result.is_err() { self.clear_session().await; }
        result
    }
    pub fn is_stopping(&self) -> bool { self.stopping.load(Ordering::SeqCst) }

    #[cfg(feature = "private-fixtures")]
    pub fn set_private_callback_adapter(&self, adapter: fn(&str, Value) -> Option<Value>) {
        *self.private_callbacks.lock().expect("private callback adapter") = Some(adapter);
    }

    pub async fn resume_after_maintenance(&self, app: &AppHandle) -> Reply {
        if self.current.lock().await.is_some() { return Err("native_closing"); }
        self.stopping.store(false, Ordering::SeqCst);
        self.connect(app).await
    }

    pub async fn stop(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.clear_session().await;
    }

    pub(super) async fn clear_session(&self) {
        if let Some(owner) = self.hotkeys.lock().await.take() { owner.stop().await; }
        self.stop_recording().await;
        self.agent.lock().await.take();
        self.clipboard.lock().await.take();
        let service = self.current.lock().await.take();
        if let Some(account) = self.account.lock().await.take() { account.stop().await; }
        if let Some(service) = service { service.stop().await; }
    }
}


pub(crate) fn deliver(app: &AppHandle, mut event: NativeEvent) {
    if event.event == "runtime.snapshot" { crate::persistence::mark_storage(&mut event.data); }
    // Synthetic account/billing UI has its own fake transport observations;
    // the empty isolated credential profile must not overwrite those fixtures.
    #[cfg(feature = "private-fixtures")]
    if std::env::var_os("GOVORUN_NATIVE_WORKER").is_some()
        && matches!(event.event.as_str(), "account.changed" | "billing.changed") { return; }
    if matches!(event.event.as_str(), "runtime.exited" | "runtime.startFailed") {
        app.state::<crate::extended_diagnostics::ExtendedDiagnostics>().record_system(app, event.event == "runtime.startFailed",
            event.data.get("code").and_then(Value::as_i64).and_then(|code| i32::try_from(code).ok()));
    }
    crate::tray::update(app, &event.event, &event.data);
    if event.event == "navigation" {
        if let Some(route) = event.data.get("route").and_then(Value::as_str) {
            let route = route.to_string();
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || { let _ = crate::windows::navigate(&handle, &route); });
        }
    }
    for window in app.webview_windows().values().filter(|window| window.label() != "recording") { let _ = window.emit("govorun:event", &event); }
}
