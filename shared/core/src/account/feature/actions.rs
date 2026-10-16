use super::*;
impl AccountFeature {
    async fn request(
        &self,
        epoch: u64,
        operation: &str,
        mut values: Value,
    ) -> Result<Value, Error> {
        values["operation"] = json!(operation);
        values["sessionGeneration"] = json!(epoch);
        let response = govorun_core::account_protocol::dispatch(&self.core, values).await?;
        let status = response["status"]
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or(Error::InvalidResponse)?;
        if !(200..300).contains(&status) {
            return Err(Error::Http(status));
        }
        Ok(response["body"].clone())
    }
    pub(super) async fn execute(
        &self,
        action: &str,
        params: Value,
        provider: Option<AuthProvider>,
        intent: AuthIntent,
        epoch: u64,
    ) -> Result<(), Error> {
        self.auth.check_intent(intent)?;
        if !self.current(epoch) {
            return Err(Error::SessionChanged);
        }
        self.native
            .publish_authority(self.core.native_projection())
            .await
            .map_err(|_| Error::StorageUnavailable)?;
        match action {
            "signIn" => {
                self.agent(
                    "account.localEffect",
                    json!({"effect":"prepareSignIn","generation":epoch}),
                )
                .await
                .map_err(|_| Error::StorageUnavailable)?;
                let url = self
                    .auth
                    .begin_reserved(intent, provider.ok_or(Error::InvalidRequest)?)
                    .await?;
                self.auth.check_intent(intent)?;
                if self.stopped.load(Ordering::SeqCst) {
                    return Err(Error::SessionChanged);
                }
                if self.open(url).await.is_err() {
                    self.auth.cancel_reserved(intent).await?;
                    return Err(Error::NetworkUnavailable);
                }
            }
            "callback" | "replaceDevice" => {
                let device = self.device().await?;
                if action == "callback" {
                    self.auth
                        .callback_reserved(
                            intent,
                            params["url"].as_str().ok_or(Error::InvalidRequest)?,
                            device,
                        )
                        .await?;
                } else {
                    self.auth
                        .replace_reserved(
                            intent,
                            params["deviceId"].as_str().ok_or(Error::InvalidRequest)?,
                            true,
                            device,
                        )
                        .await?;
                }
                let _=govorun_core::account_protocol::dispatch(&self.core,json!({"operation":"refreshBillingStatus","sessionGeneration":self.core.snapshot().session_generation})).await;
                let epoch = self.core.snapshot().session_generation;
                self.request(epoch, "accountDevices", json!({})).await?;
            }
            "cancelSignIn" => self.auth.cancel_reserved(intent).await?,
            "signOut" => self.auth.logout_reserved(intent).await?,
            "restartStorage" => {
                self.auth.restart_storage_reserved(intent).await?;
                let current = self.core.snapshot().session_generation;
                self.native
                    .publish_authority(self.core.native_projection())
                    .await
                    .map_err(|_| Error::StorageUnavailable)?;
                self.agent(
                    "account.localEffect",
                    json!({"effect":"recoverVault","generation":current}),
                )
                .await
                .map_err(|_| Error::StorageUnavailable)?;
            }
            "recoverStorage" => {
                self.core.restore(true).await?;
                let current = self.core.snapshot().session_generation;
                self.native
                    .publish_authority(self.core.native_projection())
                    .await
                    .map_err(|_| Error::StorageUnavailable)?;
                self.agent(
                    "account.localEffect",
                    json!({"effect":"recoverVault","generation":current}),
                )
                .await
                .map_err(|_| Error::StorageUnavailable)?;
            }
            "refresh" => {
                self.request(epoch, "accountProfile", json!({})).await?;
                let facts = self
                    .agent("account.localFacts", json!({}))
                    .await
                    .map_err(|_| Error::InvalidResponse)?;
                let name = facts["deviceName"]
                    .as_str()
                    .filter(|value| !value.is_empty() && value.len() <= 256)
                    .ok_or(Error::InvalidResponse)?;
                self.request(epoch, "accountUpdateDevice", json!({"deviceName":name}))
                    .await?;
                self.documents()?.request_sync();
            }
            "revokeDevice" => {
                let id = params["deviceId"].as_str().ok_or(Error::InvalidRequest)?;
                self.request(epoch, "accountRevokeDevice", json!({"deviceId":id}))
                    .await?;
            }
            "deleteAccount" => {
                let response = self
                    .request(epoch, "accountDelete", json!({"confirmed":true}))
                    .await?;
                if response["status"] != "deleted" {
                    return Err(Error::InvalidResponse);
                }
            }
            "merge" | "keepLocal" | "requestSync" => {
                use govorun_core::sync_enrollment::Choice;
                let choice = match action {
                    "merge" => Choice::Merge,
                    "keepLocal" => Choice::KeepLocal,
                    _ => Choice::RequestDecision,
                };
                self.documents()?
                    .choose(epoch, choice)
                    .await
                    .map_err(|error| {
                        if error == "account_session_changed" {
                            Error::SessionChanged
                        } else {
                            Error::StorageUnavailable
                        }
                    })?;
            }
            _ => return Err(Error::InvalidRequest),
        }
        Ok(())
    }
}
