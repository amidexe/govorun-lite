//! Production event loop. Hardware/workflow ports are injected only at composition.
//! The same loop is measured with recorded packets and memory IO in private gates.
use super::{capture, timer};
use super::recording::Lease;
use govorun_core::recording_controls::Mode;
use super::input_events::{Audio as Event, Action, Event as KeyEvent, Target};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};
pub trait State: Send + Sync {
    fn mode(&self) -> Option<Mode>;
    fn processing_count(&self) -> Option<usize>;
    fn model_deadline(&self, active: bool) -> Option<Instant>;
    fn input_changed(&self, active: bool);
}
fn now(clock: Instant) -> u64 {
    clock.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
pub enum Command<J> {
    Key(KeyEvent),
    Audio(Event<J>),
    Stop(Option<Lease>, u64),
    Cancel,
    ProcessingChanged,
    Timer(Lease, Instant, timer::Plan),
    Maintenance(oneshot::Sender<bool>),
    Resume,
    Shutdown(oneshot::Sender<()>),
}
pub async fn run<P: capture::Ports>(
    mut owner: capture::Owner<P>,
    mut receive: mpsc::UnboundedReceiver<Command<P::Job>>,
    state: std::sync::Arc<dyn State>,
    clock: Instant,
) {
    let mut schedule = timer::Schedule::default();
    let mut suspended = false;
    loop {
        let deadline = owner.deadline().filter(|deadline| *deadline > now(clock));
        let tail_at = deadline.map(|at| clock + std::time::Duration::from_nanos(at));
        let timer_at = schedule.next(owner.recording());
        let model_at = state.model_deadline(owner.active().is_some());
        let wake_at = [tail_at, timer_at, model_at].into_iter().flatten().min();
        let command = tokio::select! {
            command = receive.recv() => command,
            _ = async {
                if let Some(deadline) = wake_at {
                    tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
                } else { std::future::pending::<()>().await; }
            } => {
                match schedule.tick(Instant::now(), owner.recording()) {
                    Some(timer::Action::Warn(lease)) => owner.warning(lease),
                    Some(timer::Action::Stop(lease)) => { let _ = owner.stop(lease, now(clock)); },
                    None => {},
                }
                owner.tick(now(clock));
                state.input_changed(owner.active().is_some());
                continue;
            }
        };
        let Some(command) = command else {
            owner.cancel();
            return;
        };
        match command {
            Command::Key(event) => match (event.target, event.action) {
                (Target::Cancel, Action::Press) | (_, Action::Abort) => owner.cancel(),
                (_, Action::Press) if suspended => {}
                (_, Action::Press) => {
                    let mode = state.mode();
                    if let Some(mode) = mode {
                        let _ = owner.press(event.at_nanos, mode, event.target == Target::Plain);
                    }
                }
                (_, Action::Release) => {
                    let _ = owner.release(event.at_nanos);
                }
            },
            Command::Audio(Event::Ready { id }) => owner.ready(id),
            Command::Audio(Event::Stopped { id, job }) => owner.captured(id, job, now(clock)),
            Command::Audio(Event::Failed { id, code }) => owner.failed(id, code),
            Command::Audio(Event::FailedNative { id, code, native }) => owner.failed_native(id, code, Some(native)),
            Command::Audio(Event::Cancelled { id }) => owner.failed(id, "recording_cancelled"),
            Command::Stop(lease, at) => {
                if let Some(lease) = lease.or_else(|| owner.active()) {
                    let _ = owner.stop(lease, at);
                }
            }
            Command::Cancel => owner.cancel(),
            Command::ProcessingChanged => {}
            Command::Timer(lease, ready_at, plan) => {
                schedule.install(lease, ready_at, plan, owner.recording())
            }
            Command::Maintenance(done) => {
                let idle = !suspended && owner.active().is_none() && state.processing_count() == Some(0);
                if idle {
                    suspended = true;
                }
                let _ = done.send(idle);
            }
            Command::Resume => {
                suspended = false;
            }
            Command::Shutdown(done) => {
                owner.cancel();
                state.input_changed(owner.active().is_some());
                let _ = done.send(());
                return;
            }
        }
        state.input_changed(owner.active().is_some());
    }
}
