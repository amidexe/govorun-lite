use govorun_core::{Error, Result};
use jni::{
    objects::{GlobalRef, JObject, JString, JValue},
    JavaVM,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::oneshot;

struct Pending {
    network: bool,
    response_limit: usize,
    reply: oneshot::Sender<Result<Value>>,
}
const COMMAND_LIMIT: usize = 524_288;
// Core still limits the decoded v8 record to 4 MiB. JSON string escaping can
// double those bytes at the physical I/O edge, independently of command limits.
fn effect_limit(value: &Value) -> usize {
    if value["operation"] == "styleVault" { 9 * 1024 * 1024 } else { COMMAND_LIMIT }
}

/// Long-lived application I/O port. Java starts asynchronous work and responds
/// through JNI; no runtime thread blocks on a managed coroutine or an Activity.
pub(crate) struct SessionHost {
    vm: JavaVM,
    callback: GlobalRef,
    next: AtomicU64,
    network_closed: AtomicBool,
    pending: Mutex<HashMap<u64, Pending>>,
}

impl SessionHost {
    pub fn new(vm: JavaVM, callback: GlobalRef) -> Arc<Self> {
        Arc::new(Self {
            vm,
            callback,
            next: AtomicU64::new(1),
            network_closed: AtomicBool::new(false),
            pending: Mutex::new(HashMap::new()),
        })
    }

    fn notify(&self, method: &str, id: u64, value: Option<Value>) -> Result<()> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|_| Error::NetworkUnavailable)?;
        let result = if let Some(value) = value {
            let text = value.to_string();
            let limit = if method == "effect" {effect_limit(&value)} else {COMMAND_LIMIT};
            if text.len() > limit {
                return Err(Error::ResponseTooLarge);
            }
            env.new_string(text).and_then(|text| {
                env.call_method(
                    self.callback.as_obj(),
                    method,
                    "(JLjava/lang/String;)V",
                    &[
                        JValue::Long(id as i64),
                        JValue::Object(&JObject::from(text)),
                    ],
                )
            })
        } else {
            env.call_method(
                self.callback.as_obj(),
                method,
                "(J)V",
                &[JValue::Long(id as i64)],
            )
        };
        if env.exception_check().unwrap_or(true) {
            // This is an attached background thread, not the JNI caller's frame.
            // Never print a Java exception or private command values.
            let _ = env.exception_clear();
            return Err(Error::NetworkUnavailable);
        }
        result.map(|_| ()).map_err(|_| Error::NetworkUnavailable)
    }

    pub async fn effect(self: &Arc<Self>, command: Value, network: bool) -> Result<Value> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (reply, response) = oneshot::channel();
        let _guard = EffectGuard {
            host: self.clone(),
            id,
        };
        {
            let mut pending = self.pending.lock().map_err(|_| Error::NetworkUnavailable)?;
            if pending.len() >= 128 || network && self.network_closed.load(Ordering::SeqCst) {
                return Err(Error::NetworkUnavailable);
            }
            pending.insert(id, Pending { network, response_limit:effect_limit(&command), reply });
            // Keep start/cancel order across the JNI boundary. Managed effect()
            // only enqueues work and cannot wait for its response here.
            self.notify("effect", id, Some(command))?;
        }
        response.await.map_err(|_| Error::NetworkUnavailable)?
    }

    pub fn respond(&self, id: u64, value: Value) {
        let Some(pending) = self.pending.lock().ok().and_then(|mut all| all.remove(&id)) else {
            return;
        };
        let result = match value["error"].as_str() {
            None => Ok(value),
            Some("account_storage_unavailable") => Err(Error::StorageUnavailable),
            Some("account_storage_invalid") => Err(Error::StorageInvalid),
            Some("account_session_changed") => Err(Error::SessionChanged),
            Some("account_invalid_response") => Err(Error::InvalidResponse),
            Some("account_response_too_large") => Err(Error::ResponseTooLarge),
            Some("style_transport_security") => Err(Error::StyleTransport(govorun_core::stylization::Failure::Security)),
            Some("style_confirmed_offline") => Err(Error::StyleTransport(govorun_core::stylization::Failure::Offline)),
            Some("style_cancelled") => Err(Error::StyleTransport(govorun_core::stylization::Failure::Cancelled)),
            Some("style_deadline") => Err(Error::StyleTransport(govorun_core::stylization::Failure::Deadline)),
            Some("style_transport_other") => Err(Error::StyleTransport(govorun_core::stylization::Failure::Other)),
            Some("style_response_too_large") => Err(Error::ResponseTooLarge),
            Some("style_invalid_response") => Err(Error::InvalidResponse),
            _ => Err(Error::NetworkUnavailable),
        };
        let _ = pending.reply.send(result);
    }
    pub fn response_input(&self, env:&mut jni::JNIEnv<'_>, id:u64, input:JString<'_>) -> Result<Value> {
        let limit = self.pending.lock().map_err(|_|Error::NetworkUnavailable)?
            .get(&id).map_or(COMMAND_LIMIT,|p|p.response_limit);
        input_with_limit(env,input,limit)
    }

    pub fn complete(&self, id: u64, result: Result<Value>) {
        let value = match result {Ok(value)=>json!({"result":value}), Err(error)=>error.wire()};
        self.complete_value(id,value);
    }
    pub fn complete_style(&self, id: u64, result: Result<Value>, timing: Option<String>) {
        let mut value = match result {
            Ok(value)=>json!({"result":value}),
            Err(error)=>{
                let mut value = error.wire();
                value["stylizationFailure"] = json!(govorun_core::stylization::failure::decide(error));
                value
            }
        };
        if let Some(timing) = timing { value["clientTiming"] = json!(timing); }
        self.complete_value(id,value);
    }
    fn complete_value(&self,id:u64,value:Value) {
        if let Err(error) = self.notify("complete", id, Some(value)) {
            // A bounded framing failure still finishes the managed waiter.
            let _ = self.notify("complete", id, Some(error.wire()));
        }
    }

    pub fn close_network(&self) {
        let ids = {
            let mut pending = self.pending.lock().expect("session effects");
            self.network_closed.store(true, Ordering::SeqCst);
            let ids: Vec<_> = pending
                .iter()
                .filter_map(|(id, p)| p.network.then_some(*id))
                .collect();
            for id in &ids {
                pending.remove(id);
            }
            ids
        };
        for id in ids {
            let _ = self.notify("cancelEffect", id, None);
        }
    }
}

struct EffectGuard {
    host: Arc<SessionHost>,
    id: u64,
}
impl Drop for EffectGuard {
    fn drop(&mut self) {
        let removed = self
            .host
            .pending
            .lock()
            .ok()
            .and_then(|mut all| all.remove(&self.id));
        if removed.is_some() {
            let _ = self.host.notify("cancelEffect", self.id, None);
        }
    }
}

pub(crate) fn input(env: &mut jni::JNIEnv<'_>, input: JString<'_>) -> Result<Value> {
    input_with_limit(env,input,COMMAND_LIMIT)
}
fn input_with_limit(env: &mut jni::JNIEnv<'_>, input: JString<'_>, limit:usize) -> Result<Value> {
    if env
        .call_method(&input, "length", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|_| Error::InvalidRequest)?
        > limit as i32
    {
        return Err(Error::ResponseTooLarge);
    }
    let text: String = env
        .get_string(&input)
        .map_err(|_| Error::InvalidRequest)?
        .into();
    if text.len() > limit {
        return Err(Error::ResponseTooLarge);
    }
    serde_json::from_str(&text).map_err(|_| Error::InvalidRequest)
}
