//! The account boundary cancels anonymous I/O too, without putting account
//! credentials or identity into the gateway transport.
use super::{Failure, Gateway, Kind, Request, Response};
use crate::AccountSession;
impl AccountSession {
    pub(super) async fn send_gateway_scoped(
        &self,
        epoch: u64,
        gateway: &dyn Gateway,
        request: Request,
    ) -> std::result::Result<Response, Failure> {
        self.send_gateway_observed(epoch, gateway, request, &|_| {})
            .await
    }
    pub(super) async fn send_gateway_observed(
        &self,
        epoch: u64,
        gateway: &dyn Gateway,
        request: Request,
        observe: &(dyn Fn(super::timing::ExchangeEvent) + Send + Sync),
    ) -> std::result::Result<Response, Failure> {
        let mut boundary = self.inner.boundary.subscribe();
        let mut changes = self.subscribe();
        let reporting = request.kind() == Kind::Reliability;
        let admitted = matches!(request.kind(), Kind::Start | Kind::Status);
        let allowed = || {
            let snapshot = self.snapshot();
            self.current(epoch).is_ok()
                && snapshot.signed_in
                && snapshot.storage_available
                && !snapshot.logout_pending
                && if admitted {
                    true
                } else if reporting {
                    snapshot.access.consent_accepted
                } else {
                    self.style_authority().processing_ready
                }
        };
        if !allowed() {
            return Err(Failure::Cancelled);
        }
        let timeout = request.timeout();
        let response = tokio::select! {
            biased;
            _ = boundary.changed() => return Err(Failure::Cancelled),
            _ = async {
                loop {
                    if changes.changed().await.is_err() || !allowed() { break; }
                }
            } => return Err(Failure::Cancelled),
            result = tokio::time::timeout(timeout, gateway.send_observed(request, observe)) => {
                result.unwrap_or_else(|_| {
                    observe(super::timing::ExchangeEvent::Deadline(timeout.as_millis().min(120_000) as u64));
                    Err(Failure::Deadline)
                })
            },
        };
        if !allowed() {
            return Err(Failure::Cancelled);
        }
        response
    }
}
