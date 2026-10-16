use crate::{
    AccountRequest, AccountSession, Credentials, Error, Future, HttpResponse, Profile, Result,
    SignInHost,
};
use serde_json::Value;
use tokio::sync::watch;

pub(crate) struct SessionSignIn<'a> {
    pub session: &'a AccountSession,
    pub boundary: watch::Receiver<u64>,
}

impl SignInHost for SessionSignIn<'_> {
    fn current(&self, epoch: u64) -> Result<()> {
        self.session.current(epoch)
    }

    fn request(
        &self,
        epoch: u64,
        request: AccountRequest,
        access_token: Option<String>,
    ) -> Future<'_, Result<HttpResponse>> {
        Box::pin(
            self.session
                .send_scoped(epoch, self.boundary.clone(), request, access_token),
        )
    }

    fn device_limit(&self, epoch: u64, value: Value) -> Result<()> {
        let mut state = self.session.mutate();
        if state.epoch != epoch {
            return Err(Error::SessionChanged);
        }
        state.pending_device_limit = Some(value);
        Ok(())
    }

    fn publish(&self, epoch: u64, tokens: Credentials, profile: Profile) -> Future<'_, Result<()>> {
        Box::pin(async move {
            // The worker retains the storage gate through commit/rollback even
            // if the caller disappears while a native write is in progress.
            let session = self.session.clone();
            tokio::spawn(async move { session.commit_sign_in(epoch, tokens, profile).await })
                .await
                .map_err(|_| Error::StorageUnavailable)?
        })
    }
}
