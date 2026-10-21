//! One foreground use case. Core chooses admission/delivery; ports only
//! provide observed facts and perform effects. Text is private to this scope.
pub(crate) mod core;
pub mod capture;
mod ports;
pub mod measurement;
pub mod preparation;
pub mod queue;
pub mod recording;
pub mod transcript;
pub mod timer;
pub mod presentation;
pub use ports::{Failure, Ports, Recognition, Stage, Styled};
use serde_json::{json, Value};
use std::time::Instant;

struct Delivery {
    text: String,
    fallback: bool,
    response: Option<Styled>,
    record_completion: bool,
}

pub async fn run<P: Ports>(ports: &mut P) -> Result<(), Failure> {
    ports.progress(Stage::Recognizing);
    let result = execute(ports).await;
    ports.hide_progress();
    result
}

fn current(ports: &impl Ports) -> Result<(), Failure> {
    if ports.cancelled() {
        Err(Failure::Cancelled)
    } else {
        Ok(())
    }
}

async fn execute<P: Ports>(ports: &mut P) -> Result<(), Failure> {
    current(ports)?;
    let origin=ports.timing_origin();
    let boundary=||origin.map(measurement::offset).unwrap_or(0);
    let tail=boundary();
    let started = Instant::now();
    let recognition = ports.recognize().await?;
    let recognized=boundary();
    let input_chars=recognition.text.encode_utf16().count();
    let input_words=crate::statistics_repository::recording_session_facts(&recognition.text,0)
        .ok().and_then(|value|value["words"].as_u64()).unwrap_or(0);
    current(ports)?;
    ports
        .record_recognition(
            recognition.speech_samples,
            started.elapsed(),
            recognition.failed_chunks == 0,
        )
        .await;
    current(ports)?;
    let original = ports.dictionary(recognition.text).await;
    current(ports)?;
    if original.trim().is_empty() {
        ports.fallback();
        return Ok(());
    }
    ports
        .record_session(&original, recognition.speech_samples)
        .await;
    current(ports)?;
    let prepared=boundary();
    let delivery = match process(&original, ports).await {
        Ok(delivery) => delivery,
        Err(Failure::Cancelled) => return Err(Failure::Cancelled),
        Err(Failure::Effect(reason)) => {
            ports.failure(reason);
            Delivery {
                text: original.clone(),
                fallback: true,
                response: None,
                record_completion: false,
            }
        }
        Err(Failure::SelfHostedHttp(_)) => {
            ports.failure("requestFailed");
            Delivery {
                text: original.clone(),
                fallback: true,
                response: None,
                record_completion: false,
            }
        }
    };
    let processed=boundary();
    current(ports)?;
    let units: Vec<_> = delivery.text.encode_utf16().collect();
    let original_units: Vec<_> = original.encode_utf16().collect();
    let plan = core::evaluate(json!({"kind":"delivery","command":{"kind":"prepare",
        "text":units,"original":original_units,"marker":ports.marker(),"fallback":delivery.fallback,
        "hasOutcomePass":delivery.response.as_ref().is_some_and(|v| v.outcome_pass.is_some()),
        "latencyKnown":delivery.response.as_ref().is_some_and(|v| v.latency_bucket.as_deref().is_some_and(|v| v != "unknown"))}}));
    // A malformed local setting cannot destroy a completed transcript. Core
    // still chooses the safe unmarked delivery and visible failure effects.
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => {
            current(ports)?;
            if error == Failure::Cancelled {
                return Err(error);
            }
            ports.failure("dictation_invalid_delivery");
            core::evaluate(json!({"kind":"delivery","command":{"kind":"prepare",
                "text":units,"original":original_units,"marker":"off","fallback":true,
                "hasOutcomePass":false,"latencyKnown":false}}))?
        }
    };
    let insert = plan["insert"]
        .as_bool()
        .ok_or(Failure::Effect("dictation_invalid_delivery"))?;
    let text = plan["text"]
        .as_str()
        .ok_or(Failure::Effect("dictation_invalid_delivery"))?;
    current(ports)?;
    let inserted = if insert {
        ports.paste(format!("{text} ")).await
    } else {
        false
    };
    if insert && !delivery.fallback && delivery.text.trim() != original.trim() {
        ports.keep_original(&original);
    }
    let completed=boundary();
    current(ports)?;
    let measured=if let Some(origin)=origin {
        ports.record_measurement(measurement::Measurement::observed(origin,
            [tail,recognized,prepared,processed,completed],input_chars,input_words,original_units.len(),units.len()),inserted).await
    } else {false};
    if inserted && delivery.record_completion && !measured {ports.record_local_completion().await;}
    let effects = &plan[if inserted { "success" } else { "failure" }];
    if effects["showFallback"] == true {
        ports.fallback();
    }
    if effects["outcome"].is_string() {
        if let Some(response) = delivery.response {
            if let (Some(pass), Some(bucket)) = (response.outcome_pass, response.latency_bucket) {
                ports.report_outcome(pass, bucket, inserted, response.epoch);
            }
        }
    }
    Ok(())
}

async fn process<P: Ports>(text: &str, ports: &mut P) -> Result<Delivery, Failure> {
    let mut state = Value::Null;
    let begin = ports.facts("begin", text, 0).await?;
    let mut frame = step(&mut state, begin)?;
    let mut epoch = 0;
    let mut preparation_millis = 0;
    let mut response: Option<Styled> = None;
    loop {
        current(ports)?;
        let action = frame["action"]
            .as_str()
            .ok_or(Failure::Effect("dictation_invalid_workflow"))?;
        if action == "complete" {
            let outcome = frame["outcome"].as_str().unwrap_or("");
            let reason = frame["reason"].as_str().unwrap_or("");
            if outcome == "styled" {
                if let Some(response) = response {
                    ports.did_style();
                    return Ok(Delivery {
                        text: response.text.clone(),
                        fallback: false,
                        response: Some(response),
                        record_completion: false,
                    });
                }
            }
            if outcome == "fallback" {
                ports.failure(reason);
            }
            return Ok(Delivery {
                text: text.to_owned(),
                fallback: outcome == "fallback",
                record_completion: outcome == "local" && response.is_none(),
                response,
            });
        }
        if action == "selfHosted" {
            return self_hosted(text, ports).await;
        }
        let result = match action {
            "readOfficial" => {
                epoch = ports.session_generation();
                ports.facts(action, text, epoch).await
            }
            "prepare" => {
                let credits = frame["plan"]["requiredCredits"]
                    .as_u64()
                    .ok_or(Failure::Effect("dictation_invalid_workflow"))?;
                preparation_millis = ports.prepare(credits).await?;
                current(ports)?;
                ports.facts("prepared", text, epoch).await
            }
            "request" => {
                if ports.session_generation() != epoch {
                    Err(Failure::Effect("accountStateChanged"))
                } else {
                    ports.progress(Stage::Styling);
                    match ports.stylize(text, preparation_millis).await {
                        Ok(mut styled) => {
                            current(ports)?;
                            if ports.session_generation() != epoch {
                                return Err(Failure::Cancelled);
                            }
                            styled.epoch = epoch;
                            let event = json!({"kind":"response","input":text.encode_utf16().collect::<Vec<_>>(),
                                "output":styled.text.encode_utf16().collect::<Vec<_>>()});
                            response = Some(styled);
                            Ok(event)
                        }
                        Err(error) => Err(error),
                    }
                }
            }
            _ => ports.facts(action, text, epoch).await,
        };
        current(ports)?;
        let event = match result {
            Ok(event) => event,
            Err(Failure::Cancelled) => return Err(Failure::Cancelled),
            Err(Failure::Effect(reason)) => json!({"kind":"failure","reason":reason}),
            Err(Failure::SelfHostedHttp(_)) => json!({"kind":"failure","reason":"requestFailed"}),
        };
        frame = step(&mut state, event)?;
    }
}

async fn self_hosted<P: Ports>(text: &str, ports: &mut P) -> Result<Delivery, Failure> {
    let input: Vec<_> = text.encode_utf16().collect();
    let begin =
        core::evaluate(json!({"kind":"selfHosted","command":{"kind":"begin","input":input}}))?;
    if begin["action"] != "request" {
        return Ok(Delivery {
            text: text.to_owned(),
            fallback: false,
            response: None,
            record_completion: true,
        });
    }
    let response = ports.self_hosted(text).await;
    current(ports)?;
    let status = match &response { Err(Failure::SelfHostedHttp(status)) => Some(*status), _ => None };
    let (command, output) = match response {
        Ok(output) => (
            json!({"kind":"response","input":input,"output":output.encode_utf16().collect::<Vec<_>>()}),
            Some(output),
        ),
        Err(Failure::Cancelled) => return Err(Failure::Cancelled),
        Err(Failure::SelfHostedHttp(status)) => (json!({"kind":"failure","status":status}), None),
        Err(Failure::Effect(_)) => (json!({"kind":"failure","status":0}), None),
    };
    let decision = core::evaluate(json!({"kind":"selfHosted","command":command}))?;
    let fallback = decision["fallback"]
        .as_bool()
        .ok_or(Failure::Effect("dictation_invalid_workflow"))?;
    if fallback {
        ports.record_self_host_failure(status).await;
        ports.failure(decision["diagnostic"].as_str().unwrap_or("failed"));
    }
    let delivered = if decision["action"] == "output" {
        output.ok_or(Failure::Effect("dictation_invalid_workflow"))?
    } else {
        text.to_owned()
    };
    Ok(Delivery {
        text: delivered,
        fallback,
        response: None,
        // Self-hosted success has no official timing event to record the action.
        record_completion: !fallback,
    })
}

fn step(state: &mut Value, event: Value) -> Result<Value, Failure> {
    let frame = core::evaluate(json!({"kind":"workflow","state":state,"event":event}))?;
    *state = frame
        .get("state")
        .cloned()
        .ok_or(Failure::Effect("dictation_invalid_workflow"))?;
    Ok(frame)
}

pub mod inputs;
pub mod input_events;
pub mod settings;
pub mod resources;
pub mod actor;
pub mod hook_registration;
pub mod feedback_worker;

/// Recording constants used when the host composes physical input ports.
pub fn constants() -> Result<Value, Failure> { core::evaluate(json!({"kind":"constants"})) }
