//! Closed path and header set. Neither bearer credentials nor arbitrary headers
//! can be represented by this request; raw text never reaches Accounts.
use super::reliability::Batch;
use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Start,
    Status,
    Reconcile,
    Retire,
    Availability,
    Outcome,
    Reliability,
}
pub struct Request {
    kind: Kind,
    body: Option<Value>,
    timeout: Duration,
    request_id: Option<String>,
    job_token: Option<String>,
    outcome_pass: Option<String>,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Payload {
    text: String,
    style: String,
    instruction: Option<String>,
    input_mode: String,
    punctuation_mode: String,
    reduced_punctuation: bool,
    emoji_enabled: bool,
    emoji_mode: String,
    profanity_filter_enabled: bool,
    client_reliability: Option<Batch>,
    style_passes: Vec<String>,
    /// Говорун.Эхо's note, set only by Android's Echo entry: "note" (light laid out as a note) or "header" (name and
    /// description over any style). Every other client leaves it out.
    echo_note: Option<String>,
    /// "note" with an Echo note only: the server's note window instead of the length table. "digest" with the mark
    /// "digest" only: Echo's итоги of a long note, their own length and price.
    request_class: Option<String>,
    /// The product the request is for, as Govorun's entry labelled it ("echo"); absent for a dictation. A note mark
    /// always comes with its product (crate::style_text::product).
    product: Option<String>,
}
impl Request {
    fn new(kind: Kind, body: Option<Value>, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() || timeout > Duration::from_secs(120) {
            return Err(Error::InvalidRequest);
        }
        if body.as_ref().is_some_and(|v| v.to_string().len() > 196_608) {
            return Err(Error::StyleRequestTooLarge);
        }
        Ok(Self {
            kind,
            body,
            timeout,
            request_id: None,
            job_token: None,
            outcome_pass: None,
        })
    }
    pub fn start(body: Value, request_id: String, timeout: Duration) -> Result<Self> {
        crate::style_text::product(&body)?;
        let payload: Payload =
            serde_json::from_value(body.clone()).map_err(|_| Error::InvalidRequest)?;
        let _ = &payload.product;
        let admission = crate::style_text::admit_class(payload.text.encode_utf16().collect(),
            payload.request_class.as_deref())?;
        if !crate::request::uuid(&request_id)
            || !["formal", "light", "concise", "custom"].contains(&payload.style.as_str())
            || !["hold", "confirm"].contains(&payload.input_mode.as_str())
            || !["less", "normal"].contains(&payload.punctuation_mode.as_str())
            || !["off", "restrained", "lively", "bright"].contains(&payload.emoji_mode.as_str())
            || payload.reduced_punctuation != (payload.punctuation_mode == "less")
            || payload.emoji_enabled != (payload.emoji_mode != "off")
            || payload.style_passes.len() as u64
                != admission["requiredCredits"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?
        {
            return Err(Error::InvalidRequest);
        }
        crate::credit_recovery_wire::request(payload.style_passes)?;
        if payload.style == "custom"
            && payload.instruction.as_ref().is_none_or(|s| {
                !(3..=crate::style_definition::MAX_INSTRUCTION).contains(&s.trim().chars().count())
            })
            || payload.style != "custom" && payload.instruction.is_some()
        {
            return Err(Error::InvalidRequest);
        }
        let digest = Some(crate::style_text::DIGEST);
        if payload.echo_note.as_deref().is_some_and(|mark| {
            !(mark == "header" || (mark == "note" || Some(mark) == digest) && payload.style == "light")
        }) || payload.request_class.as_deref().is_some_and(|class| {
            !(class == "note" && payload.echo_note.is_some() && payload.echo_note.as_deref() != digest
                || Some(class) == digest && payload.echo_note.as_deref() == digest)
        }) || payload.echo_note.as_deref() == digest && payload.request_class.as_deref() != digest {
            return Err(Error::InvalidRequest);
        }
        if let Some(batch) = payload.client_reliability {
            batch.validate()?;
        }
        let _ = payload.profanity_filter_enabled;
        let mut request = Self::new(Kind::Start, Some(body), timeout)?;
        request.request_id = Some(request_id);
        Ok(request)
    }
    pub fn status(job_token: String, timeout: Duration) -> Result<Self> {
        header(&job_token, 16_384)?;
        let mut request = Self::new(Kind::Status, None, timeout)?;
        request.job_token = Some(job_token);
        Ok(request)
    }
    pub fn reconcile(passes: Vec<String>) -> Result<Self> {
        Self::new(
            Kind::Reconcile,
            Some(crate::credit_recovery_wire::request(passes)?),
            Duration::from_secs(6),
        )
    }
    pub fn retire(passes: Vec<String>) -> Result<Self> {
        Self::new(
            Kind::Retire,
            Some(crate::credit_recovery_wire::request(passes)?),
            Duration::from_secs(6),
        )
    }
    pub fn availability() -> Result<Self> {
        Self::new(Kind::Availability, None, Duration::from_secs(4))
    }
    pub fn outcome(pass: String, outcome: &str, latency: &str) -> Result<Self> {
        header(&pass, 16_384)?;
        if !["delivered", "insertion_failed"].contains(&outcome)
            || !["lt1s", "1to2s", "2to5s", "5to10s", "10to20s", "gte20s"].contains(&latency)
        {
            return Err(Error::InvalidRequest);
        }
        let mut request = Self::new(
            Kind::Outcome,
            Some(json!({"outcome":outcome,"latencyBucket":latency})),
            Duration::from_secs(2),
        )?;
        request.outcome_pass = Some(pass);
        Ok(request)
    }
    pub fn reliability(batch: Batch) -> Result<Self> {
        batch.validate()?;
        let body = serde_json::to_value(batch).map_err(|_| Error::InvalidRequest)?;
        if body.to_string().len() > 2_048 {
            return Err(Error::InvalidRequest);
        }
        Self::new(Kind::Reliability, Some(body), Duration::from_secs(4))
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn path(&self) -> &'static str {
        match self.kind {
            Kind::Start => "/v2/stylize",
            Kind::Status => "/v1/stylize-status",
            Kind::Reconcile => "/v1/text-credits/reconcile",
            Kind::Retire => "/v1/text-credits/retire",
            Kind::Availability => "/v1/availability",
            Kind::Outcome => "/v1/client-outcomes",
            Kind::Reliability => "/v1/client-reliability",
        }
    }
    pub fn body(&self) -> Option<&Value> {
        self.body.as_ref()
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    pub fn prefer_header(&self) -> Option<&'static str> {
        match self.kind {
            Kind::Start => Some("respond-async, wait=2"),
            Kind::Status => Some("wait=1"),
            _ => None,
        }
    }
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
    pub fn job_token(&self) -> Option<&str> {
        self.job_token.as_deref()
    }
    pub fn outcome_pass(&self) -> Option<&str> {
        self.outcome_pass.as_deref()
    }
}
fn header(value: &str, max: usize) -> Result<()> {
    if value.is_empty() || value.len() > max || value.bytes().any(|b| !b.is_ascii_graphic()) {
        Err(Error::InvalidRequest)
    } else {
        Ok(())
    }
}
