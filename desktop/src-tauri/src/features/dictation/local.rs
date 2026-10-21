//! Concrete Rust recognition/insertion effects for AccountHost's foreground
//! use case. Neither text nor OS target identities enter public UI events.
use crate::account_host::AccountHost;
use crate::account_host::{OsContext, OsEffects};
use crate::features::dictation::{queue::Cancellation, Failure, Recognition, Stage};
use crate::features::dictation::{
    queue::{Completion, Queue},
    run,
};
#[cfg(target_os = "macos")]
use govorun_os_runtime::insertion::Mac as Physical;
#[cfg(target_os = "windows")]
use govorun_os_runtime::insertion::Windows as Physical;
use govorun_os_runtime::{
    inference::Job,
    insertion::{Inserter, Target},
    recognition,
};
use std::sync::{Arc, Mutex};

pub struct Work {
    job: Mutex<Option<Job>>,
    token: recognition::Cancellation,
    origin: Option<crate::features::dictation::measurement::Origin>,
}
/// Windows captures a window at recording admission; macOS captures the
/// foreground at insertion. These are physical target semantics, not rules.
pub struct Destination {
    #[cfg(target_os = "windows")]
    window: Target,
}
impl Destination {
    pub fn capture() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            window: Target::capture(),
        }
    }
    fn at_paste(&self) -> Target {
        #[cfg(target_os = "windows")]
        {
            self.window
        }
        #[cfg(target_os = "macos")]
        {
            Target::capture()
        }
    }
    pub fn application(&self) -> Option<String> {
        self.at_paste().application()
    }
}
impl Work {
    pub fn new(job: Job) -> Self {
        let token = job.cancellation();
        Self {
            job: Mutex::new(Some(job)),
            token,
            origin: None,
        }
    }
    pub fn cancel(&self) {
        self.token.cancel();
    }
    async fn recognize(self: &Arc<Self>) -> Result<Recognition, Failure> {
        if self.token.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        let job = self
            .job
            .lock()
            .map_err(|_| Failure::Effect("recognition_worker_failed"))?
            .take()
            .ok_or(Failure::Effect("recognition_job_consumed"))?;
        let output = tokio::task::spawn_blocking(move || job.finish())
            .await
            .map_err(|_| Failure::Effect("recognition_worker_failed"))?;
        if self.token.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        let output = output.map_err(|code| {
            if code == "recording_cancelled" {
                Failure::Cancelled
            } else {
                Failure::Effect(code)
            }
        })?;
        let text = crate::features::dictation::transcript::assemble(output.fragments, || {
            self.token.is_cancelled()
        })?;
        Ok(Recognition {
            text,
            speech_samples: output.speech_samples,
            failed_chunks: output.failed_chunks,
        })
    }
}
/// The production presenter maps only stages/closed failures to the existing
/// indicator and diagnostic view. It never receives clipboard/text.
pub trait Presentation: Send + Sync {
    fn progress(&self, stage: Stage);
    fn style_progress(&self, stage: govorun_core::stylization::foreground::Stage);
    fn hide(&self);
    fn fallback(&self);
    fn failure(&self, code: &'static str);
    fn insertion_failure(&self, code: &'static str) { self.failure(code); }
}
pub struct Effects {
    connection: Option<crate::prepared_connection::Prepared>,
    context: OsContext,
    work: Arc<Work>,
    insertion: Arc<Inserter<Physical>>,
    destination: Destination,
    keep_clipboard: bool,
    cancellation: Cancellation,
    presentation: Arc<dyn Presentation>,
}
impl Effects {
    pub fn new(
        context: OsContext,
        work: Arc<Work>,
        insertion: Arc<Inserter<Physical>>,
        destination: Destination,
        keep_clipboard: bool,
        cancellation: Cancellation,
        presentation: Arc<dyn Presentation>,
    ) -> Self {
        Self {
            connection: None,
            context,
            work,
            insertion,
            destination,
            keep_clipboard,
            cancellation,
            presentation,
        }
    }
}
impl OsEffects for Effects {
    fn timing_origin(&self) -> Option<crate::features::dictation::measurement::Origin> {self.work.origin}
    fn context(&self) -> &OsContext {
        &self.context
    }
    fn prepared_request_id(&self) -> Option<String> {
        self.connection.as_ref().and_then(|v| v.prepared_id())
    }
    async fn recognize(&mut self) -> Result<Recognition, Failure> {
        self.work.recognize().await
    }
    fn keep_original(&mut self, original: &str) {
        crate::last_dictation::keep_original(original);
    }
    async fn paste(&mut self, text: String) -> bool {
        // Before the attempt: a dictation that lands outside a text field can be copied again.
        crate::last_dictation::remember(&text);
        let insertion = self.insertion.clone();
        let cancellation = self.cancellation.clone();
        let token = self.work.token.clone();
        let keep = self.keep_clipboard;
        // Windows retains the original recording target. macOS follows its
        // existing paste-time foreground guard, including during helper IO.
        let target = self.destination.at_paste();
        let result = tokio::task::spawn_blocking(move || {
            insertion.paste(&text, &target, keep, &|| {
                cancellation.is_cancelled() || token.is_cancelled()
            })
        })
        .await;
        match result {
            Ok(outcome) => {
                if let Some(code) = outcome.failure {
                    if code != "insertion_cancelled" {
                        self.presentation.insertion_failure(code);
                    }
                }
                outcome.sent
            }
            Err(_) => {
                self.presentation.insertion_failure("insertion_worker_failed");
                false
            }
        }
    }
    fn progress(&self, stage: Stage) {
        self.presentation.progress(stage);
    }
    fn style_progress(
        &self,
    ) -> Arc<dyn Fn(govorun_core::stylization::foreground::Stage) + Send + Sync> {
        let presentation = self.presentation.clone();
        Arc::new(move |stage| presentation.style_progress(stage))
    }
    fn hide_progress(&self) {
        self.presentation.hide();
    }
    fn fallback(&self) {
        self.presentation.fallback();
    }
    fn failure(&self, code: &'static str) {
        self.presentation.failure(code);
    }
}

/// One processor/inserter per host. A cancelled old queue cannot bypass the
/// clipboard lock, and another capture may start while recognition is pending.
pub struct Processor {
    account: Arc<AccountHost>,
    insertion: Arc<Inserter<Physical>>,
    queue: Arc<Queue<Work>>,
}
impl Processor {
    pub fn new(
        account: Arc<AccountHost>,
        insertion: Arc<Inserter<Physical>>,
        count_changed: Arc<dyn Fn(usize) + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            account,
            insertion,
            queue: Queue::new(Arc::new(Work::cancel), count_changed),
        })
    }
    pub fn count(&self) -> usize {
        self.queue.count()
    }
    pub fn cancel(&self) {
        self.queue.cancel_all();
    }
    pub async fn recording_timer(
        &self,
        enabled: bool,
        minutes: u32,
    ) -> Result<crate::features::dictation::timer::Plan, Failure> {
        self.account.recording_timer(enabled, minutes).await
    }
    pub(crate) fn prepare_capture(&self) -> crate::prepared_connection::Prepared {
        let connection = crate::prepared_connection::Prepared::start(self.account.clone());
        let account = self.account.clone();
        tokio::spawn(async move {
            let _ = account
                .prepare_dictation(crate::features::dictation::preparation::Phase::Start)
                .await;
        });
        connection
    }
    pub fn submit(
        self: &Arc<Self>,
        job: Job,
        context: OsContext,
        destination: Destination,
        keep_clipboard: bool,
        presentation: Arc<dyn Presentation>,
        connection: Option<crate::prepared_connection::Prepared>,
        origin: Option<crate::features::dictation::measurement::Origin>,
    ) -> Result<Completion, &'static str> {
        let owner = self.clone();
        let mut work=Work::new(job);work.origin=origin;
        self.queue
            .enqueue(work, move |work, cancellation| async move {
                let mut effects = Effects::new(
                    context,
                    work,
                    owner.insertion.clone(),
                    destination,
                    keep_clipboard,
                    cancellation.clone(),
                    presentation.clone(),
                );
                effects.connection = connection;
                match owner.account.dictation_attempt(effects, cancellation) {
                    Ok(mut attempt) => match run(&mut attempt).await {
                        Ok(()) | Err(Failure::Cancelled) => {}
                        Err(Failure::Effect(code)) => presentation.failure(code),
                        Err(Failure::SelfHostedHttp(_)) => presentation.failure("requestFailed"),
                    },
                    Err(Failure::Cancelled) => {}
                    Err(_) => presentation.failure("dictation_unavailable"),
                }
            })
    }
}
