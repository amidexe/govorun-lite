//! Closed indicator view. A recording takes visual priority over older
//! queued recognition; cancelled and completed jobs cannot redraw a new capture.
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Idle,
    Starting,
    Recording,
    Processing,
    Fallback,
}
#[derive(Clone, Copy, Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Recording,
    Recognizing,
    CheckingConnection,
    Accepted,
    Processing,
}
#[derive(Clone, Copy, Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Issue {
    CaptureUnavailable,
    ProcessingFailed,
    TimerUnavailable,
}
#[derive(Clone, Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub phase: Phase,
    pub stage: Stage,
    pub warning: bool,
    pub issue: Option<Issue>,
}
#[derive(Clone, Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub revision: u64,
    #[serde(flatten)]
    pub snapshot: Snapshot,
}
struct Capture {
    id: u64,
    ready: bool,
    warning: bool,
}
#[derive(Default)]
pub struct State {
    last_admitted: u64,
    cancelled_through: u64,
    capture: Option<Capture>,
    jobs: BTreeMap<u64, Stage>,
    fallback: Option<u64>,
    last_style_failure: Option<u64>,
    issue: Option<(u64, Issue)>,
    revision: u64,
    published: Option<Snapshot>,
}
impl State {
    pub fn restart(&mut self) {
        let revision = self.revision;
        let published = self.published.clone();
        *self = Self {
            revision,
            published,
            ..Self::default()
        };
    }
    pub fn admit(&mut self, id: u64) {
        if id <= self.last_admitted {
            return;
        }
        self.last_admitted = id;
        self.capture = Some(Capture {
            id,
            ready: false,
            warning: false,
        });
        self.fallback = None;
        self.issue = None;
    }
    pub fn ready(&mut self, id: u64) {
        if let Some(capture) = self.capture.as_mut().filter(|c| c.id == id) {
            capture.ready = true;
        }
    }
    pub fn stop(&mut self, id: u64) {
        if self.capture.as_ref().is_some_and(|c| c.id == id) {
            self.capture = None;
            self.jobs.insert(id, Stage::Recognizing);
        }
    }
    pub fn progress(&mut self, id: u64, stage: Stage) {
        if let Some(job) = self.jobs.get_mut(&id) {
            *job = stage;
        }
    }
    pub fn complete(&mut self, id: u64) {
        let completed = self.jobs.remove(&id).is_some();
        // A successful short/local dictation supersedes the previous attempt's
        // warning too. This clears history, without claiming online readiness.
        if completed && id == self.last_admitted && self.last_style_failure != Some(id) {
            self.last_style_failure = None;
        }
    }
    pub fn fallback(&mut self, id: u64) {
        if id == self.last_admitted && id > self.cancelled_through {
            self.fallback = Some(id);
        }
    }
    pub fn failed_capture(&mut self, id: u64) {
        // Permission/startup can fail before the audio worker admits this attempt.
        if id > self.last_admitted {
            self.admit(id);
        }
        // Stop already reserves a recognition row while input is draining.
        // A later capture error must remove that row too: no job will complete it.
        let stopped = self.jobs.remove(&id).is_some();
        let capturing = self.capture.as_ref().is_some_and(|c| c.id == id);
        if capturing {
            self.capture = None;
        }
        if stopped || capturing {
            self.fallback(id);
        }
    }
    pub fn warning(&mut self, id: u64) {
        if let Some(capture) = self.capture.as_mut().filter(|c| c.id == id) {
            capture.warning = true;
        }
    }
    pub fn issue(&mut self, id: u64, issue: Issue) {
        if id == self.last_admitted && id > self.cancelled_through {
            if issue == Issue::ProcessingFailed && self.jobs.get(&id).is_some_and(|stage|
                matches!(stage, Stage::CheckingConnection | Stage::Accepted | Stage::Processing)) {
                self.last_style_failure = Some(id);
            }
            self.issue = Some((id, issue));
        }
    }
    pub fn clear_fallback(&mut self, id: u64) {
        if self.fallback == Some(id) {
            self.fallback = None;
            if !self.issue.is_some_and(|(_, issue)| issue == Issue::CaptureUnavailable) {
                self.issue = None;
            }
        }
    }
    pub fn cancel(&mut self) {
        self.cancelled_through = self.last_admitted;
        self.capture = None;
        self.jobs.clear();
        self.fallback = None;
        self.issue = None;
    }
    /// A local card survives the short capsule animation. No text or network state.
    pub fn style_status(&self) -> &'static str {
        if self.jobs.values().any(|stage| matches!(stage,
            Stage::CheckingConnection | Stage::Accepted | Stage::Processing)) { "processing" }
        else if self.last_style_failure.is_some() { "failed" } else { "idle" }
    }
    pub fn snapshot(&self) -> Snapshot {
        if let Some(capture) = &self.capture {
            Snapshot {
                phase: if capture.ready {
                    Phase::Recording
                } else {
                    Phase::Starting
                },
                stage: Stage::Recording,
                warning: capture.warning,
                issue: self.issue.map(|(_, issue)| issue),
            }
        } else if let Some((_, stage)) = self.jobs.first_key_value() {
            Snapshot {
                phase: Phase::Processing,
                stage: *stage,
                warning: false,
                issue: self.issue.map(|(_, issue)| issue),
            }
        } else {
            Snapshot {
                phase: if self.fallback.is_some() {
                    Phase::Fallback
                } else {
                    Phase::Idle
                },
                stage: Stage::Recording,
                warning: false,
                issue: self.issue.map(|(_, issue)| issue),
            }
        }
    }
    pub fn changed(&mut self) -> Option<Frame> {
        let snapshot = self.snapshot();
        if self.published.as_ref() == Some(&snapshot) {
            return None;
        }
        self.revision = self.revision.checked_add(1)?;
        self.published = Some(snapshot.clone());
        Some(Frame {
            revision: self.revision,
            snapshot,
        })
    }
    pub fn current_frame(&self) -> Frame {
        Frame {
            revision: self.revision,
            snapshot: self.snapshot(),
        }
    }
}
