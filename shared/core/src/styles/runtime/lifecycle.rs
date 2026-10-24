//! Runtime composition of the verified common owners, constructed after account
//! restoration. Physical connectivity and selection are supplied by host ports.
use super::{availability, commands::random_id, reconnect, reporting, usage, workers, Stylization};
use govorun_core::{
    stylization::{reconnect::Step, scheduling::Trigger},
    Error, Result,
};
use std::sync::{atomic::Ordering, Arc};
use tokio::time::Duration;

impl Stylization {
    pub fn start_background(
        self: &Arc<Self>,
        enabled: workers::Enabled,
        online: reporting::Online,
        initially_online: bool,
        changed: availability::Changed,
    ) -> Result<()> {
        self.start_workers(enabled.clone(), online.clone())?;
        self.reserve_cadence
            .set(super::cadence::Worker::start(self, enabled.clone(), online.clone()))
            .map_err(|_| Error::InvalidRequest)?;
        self.start_reporting(online.clone())?;
        let weak = Arc::downgrade(self);
        self.usage_worker
            .set(usage::Worker::start(
                self.account.clone(),
                Arc::new(move |epoch| {
                    let weak = weak.clone();
                    Box::pin(async move {
                        weak.upgrade()
                            .ok_or(Error::SessionChanged)?
                            .sync_usage(epoch)
                            .await
                    })
                }),
                online,
            ))
            .map_err(|_| Error::InvalidRequest)?;
        let weak = Arc::downgrade(self);
        self.availability_worker
            .set(availability::Worker::start(
                self.account.clone(),
                self.foreground.availability().clone(),
                enabled.clone(),
                Arc::new(move || {
                    let weak = weak.clone();
                    Box::pin(async move {
                        let owner = weak.upgrade().ok_or(Error::SessionChanged)?;
                        owner.current(owner.account.snapshot().session_generation)?;
                        owner.foreground.probe_availability().await
                    })
                }),
                changed,
            ))
            .map_err(|_| Error::InvalidRequest)?;
        let weak = Arc::downgrade(self);
        let account = self.account.clone();
        self.reconnect_worker
            .set(reconnect::Worker::start(
                self.account.clone(),
                Arc::new(move |epoch, step| {
                    let weak = weak.clone();
                    let enabled = enabled.clone();
                    Box::pin(async move {
                        let Some(owner) = weak.upgrade() else {
                            return false;
                        };
                        if owner.current(epoch).is_err() {
                            return false;
                        }
                        match step {
                            Step::Reconcile => {
                                owner.reconcile_for_reconnect(epoch).await.unwrap_or(false)
                            }
                            Step::Refresh => {
                                owner
                                    .account
                                    .refresh_for_activity(Duration::ZERO)
                                    .await
                                    .is_ok()
                                    && owner.bind_profile().await.is_ok()
                            }
                            Step::Warm => {
                                match enabled().await {
                                    Ok(false) => true,
                                    Err(_) => false,
                                    Ok(true) => {
                                        let Ok(id) = random_id() else {
                                            return false;
                                        };
                                        let result = owner.prepare(epoch, false, true, 1, id).await;
                                        // Additional reserve opportunities remain owned by
                                        // the common scheduler; no text is replayed.
                                        let _ = owner.maintain(epoch, Trigger::Maintenance, true);
                                        result.is_ok_and(|v| v.ready && !v.requires_recovery)
                                    }
                                }
                            }
                            _ => false,
                        }
                    })
                }),
                Arc::new(move || account.snapshot().authority.processing_ready),
                initially_online,
            ))
            .map_err(|_| Error::InvalidRequest)?;
        Ok(())
    }
    async fn reconcile_for_reconnect(&self, epoch: u64) -> Result<bool> {
        self.current(epoch)?;
        self.bind_profile().await?;
        if !self.foreground.has_blocking_credit(epoch).await? {
            return Ok(true);
        }
        let workers = self.workers.get().ok_or(Error::InvalidRequest)?;
        let mut completed = workers.subscribe_credit();
        workers.credit(epoch, Trigger::Demand, true, 0)?;
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                self.current(epoch)?;
                if !self.foreground.has_blocking_credit(epoch).await? {
                    return Ok(true);
                }
                completed
                    .changed()
                    .await
                    .map_err(|_| Error::SessionChanged)?;
            }
        })
        .await;
        result.unwrap_or(Ok(false))
    }
    pub fn connectivity_changed(&self, online: bool) -> Result<()> {
        // Hosts may repeat "online" after every account response. Only a change
        // is news: a repeat must not restart reconnect, probes or the reserve.
        let previous = self.connectivity.swap(if online { 2 } else { 1 }, Ordering::AcqRel);
        if online && previous == 2 { return Ok(()); }
        if !online { self.foreground.close_connection(None); }
        self.current(self.account.snapshot().session_generation)?;
        if let Some(reporter) = self.reporter.get() {
            reporter.connectivity_changed();
        }
        if let Some(worker) = self.usage_worker.get() {
            worker.connectivity_changed(online);
        }
        if !online {
            if let Some(worker) = self.availability_worker.get() {
                worker.suspend();
            }
        } else {
            self.foreground.availability().demand()?;
        }
        if online {
            if let Some(worker) = self.reserve_cadence.get() {
                worker.wake(self.account.snapshot().session_generation);
            }
        }
        if let Some(worker) = self.reconnect_worker.get() {
            worker.connectivity(online)?;
        }
        Ok(())
    }
}
