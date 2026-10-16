use super::*;
impl SelfHost {
    pub(super) async fn run(
        self: Arc<Self>,
        work: Work,
        provider: Provider,
        cancellation: Cancellation,
    ) {
        let outcome = match work {
            Work::Connect(key) => {
                // Protected-store effects finish under their lease even after
                // cancellation. No new provider/operation is admitted meanwhile.
                if !key.is_empty() && !self.write_key(provider, &key).await {
                    Outcome::KeyFailed
                } else if cancellation.is_cancelled() {
                    Outcome::Cancelled
                } else {
                    let configuration = self.load().await;
                    let (state, key) = self.key(provider, true).await;
                    match configuration {
                        Ok((_, stored))
                            if stored.settings.admit_request(stored.format, state).is_ok()
                                && self.controller.lock().unwrap().expected().as_ref()
                                    == Some(&stored.settings.configuration) =>
                        {
                            let verify = self.network.complete(
                                &stored.settings.configuration,
                                key.as_deref(),
                                "это синтетическая проверка подключения и вашего промпта",
                            );
                            tokio::select! {
                                _=cancellation.cancelled() => Outcome::Cancelled,
                                result=verify => if result.is_ok() {Outcome::Success} else {Outcome::CheckFailed},
                            }
                        }
                        _ => Outcome::CheckFailed,
                    }
                }
            }
            Work::RemoveKey => {
                if self.write_key(provider, "").await {
                    Outcome::Success
                } else {
                    Outcome::KeyFailed
                }
            }
            Work::Clear => {
                let mut success = true;
                for provider in [Provider::Ollama, Provider::Openai, Provider::Gemini] {
                    if !self.write_key(provider, "").await {
                        success = false;
                        break;
                    }
                }
                if success {
                    Outcome::Success
                } else {
                    Outcome::ClearFailed
                }
            }
            Work::None => return,
        };
        let gate = self.gate.lock().await;
        if let Ok((document, mut stored)) = self.load().await {
            let (key, _) = self
                .key(stored.settings.configuration.provider, false)
                .await;
            let outcome = if cancellation.is_cancelled() {
                Outcome::Cancelled
            } else {
                outcome
            };
            let mut next = self.controller.lock().unwrap().clone();
            next.finish(&mut stored, key, outcome);
            if self.save(&document, stored).await.is_err() {
                self.controller.lock().unwrap().storage_failed();
                // A failed CAS is visible; never retry a stale enable or deletion.
                (self.events)(
                    "selfhost.error",
                    json!({"code":"self_host_storage_unavailable"}),
                );
            } else {
                *self.controller.lock().unwrap() = next;
            }
        } else {
            self.controller.lock().unwrap().storage_failed();
            (self.events)(
                "selfhost.error",
                json!({"code":"self_host_storage_unavailable"}),
            );
        }
        self.cancellation.lock().unwrap().take();
        drop(gate);
        self.publish().await;
        if !self.stopped.load(Ordering::Acquire) {
            (self.events)("styles.changed", Value::Null);
        }
    }
}
