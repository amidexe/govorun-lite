use crate::{
    session_host::{input, SessionHost},
    session_runtime::{lookup, SessionBridge},
};
use govorun_core::{Error, Result};
use jni::{
    objects::{JObject, JString},
    sys::{jint, jlong, jstring},
    JNIEnv,
};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn boundary<T: Default>(
    env: &mut JNIEnv<'_>,
    call: impl FnOnce(&mut JNIEnv<'_>) -> Result<T>,
) -> T {
    match catch_unwind(AssertUnwindSafe(|| call(env))) {
        Ok(Ok(value)) => value,
        result => {
            let error = match result {
                Ok(Err(error)) => error.code(),
                _ => "business_bridge_unavailable",
            };
            if !env.exception_check().unwrap_or(true) {
                let _ = env.throw_new("java/lang/IllegalStateException", error);
            }
            T::default()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedAccountNative_create(
    mut env: JNIEnv,
    _receiver: JObject,
    terms: jint,
    origin: JString,
    callback: JObject,
) -> jlong {
    boundary(&mut env, |env| {
        let terms = u32::try_from(terms).map_err(|_| Error::InvalidRequest)?;
        let origin: String = env
            .get_string(&origin)
            .map_err(|_| Error::InvalidRequest)?
            .into();
        if origin.len() > 2048 {
            return Err(Error::InvalidRequest);
        }
        let host = SessionHost::new(
            env.get_java_vm().map_err(|_| Error::StorageUnavailable)?,
            env.new_global_ref(callback)
                .map_err(|_| Error::StorageUnavailable)?,
        );
        SessionBridge::create(host, terms, &origin).map(|id| id as i64)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedAccountNative_start(
    mut env: JNIEnv,
    _receiver: JObject,
    handle: jlong,
    call: jlong,
    command: JString,
) {
    boundary(&mut env, |env| {
        let bridge = lookup(handle as u64)?;
        let result = input(env, command).and_then(|command| bridge.start(call as u64, command));
        if let Err(error) = result {
            bridge.host.complete(call as u64, Err(error));
        }
        Ok(())
    });
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedAccountNative_respond(
    mut env: JNIEnv,
    _receiver: JObject,
    handle: jlong,
    effect: jlong,
    response: JString,
) {
    boundary(&mut env, |env| {
        // A response racing resource closure is discarded, never sent to a
        // different bridge even if another repository has since been created.
        if let Ok(bridge) = lookup(handle as u64) {
            let value = bridge.host.response_input(env,effect as u64,response).unwrap_or_else(|error| error.wire());
            bridge.host.respond(effect as u64, value);
        }
        Ok(())
    });
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedAccountNative_close(
    mut env: JNIEnv,
    _receiver: JObject,
    handle: jlong,
    call: jlong,
) {
    boundary(&mut env, |_| {
        let bridge = lookup(handle as u64)?;
        if let Err(error) = bridge.close(handle as u64, call as u64) {
            bridge.host.complete(call as u64, Err(error));
        }
        Ok(())
    });
}
