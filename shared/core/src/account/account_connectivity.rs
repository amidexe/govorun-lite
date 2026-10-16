//! OS notifications wake the existing Core workers; no platform retry policy.
use super::AccountHost;
use crate::agent::AgentClient;
use std::sync::{atomic::Ordering, Arc};
#[derive(Clone, Copy)]
struct Snapshot {
    online: bool,
    revision: u64,
    recovery_revision: u64,
}
impl AccountHost {
    pub(super) fn start_connectivity(
        self: &Arc<Self>,
        agent: &AgentClient,
    ) -> Result<(), &'static str> {
        let (sender, mut receiver) = tokio::sync::watch::channel::<Option<Snapshot>>(None);
        agent.bind_connectivity(Arc::new(move |online, revision, recovery_revision| {
            sender.send_if_modified(|current| {
                if current.is_some_and(|value| value.revision >= revision) {
                    return false;
                }
                *current = Some(Snapshot {
                    online,
                    revision,
                    recovery_revision,
                });
                true
            });
        }))?;
        let weak = Arc::downgrade(self);
        let native = self
            .native
            .get()
            .ok_or("account_storage_unavailable")?
            .clone();
        let task = tokio::spawn(async move {
            let mut recovered = 0;
            loop {
                let event = { *receiver.borrow_and_update() };
                if let Some(event) = event {
                    let Some(host) = weak.upgrade() else { break };
                    if host.stopping.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Some(owner) = host.stylization.get() {
                        let _ = owner.connectivity_changed(event.online);
                    }
                    if event.online && event.recovery_revision > recovered {
                        if let Ok(documents) = host.documents_owner() {
                            documents.request_sync();
                        }
                    }
                    recovered = event.recovery_revision;
                }
                tokio::select! {
                    _=native.wait_closed()=>break,
                    changed=receiver.changed()=>if changed.is_err(){break;},
                }
            }
        });
        if self.connectivity_task.set(task.abort_handle()).is_err() {
            task.abort();
            return Err("network_already_composed");
        }
        Ok(())
    }
}
