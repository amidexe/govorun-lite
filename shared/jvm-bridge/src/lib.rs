use jni::{
    objects::{JObject, JString},
    sys::jstring,
    JNIEnv,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
};
mod documents;
mod session_host;
mod session_ports;
mod session_style_ports;
mod session_styles;
mod session_timing;
mod session_runtime;
mod session_jni;

/// JNI only converts values. The same core entrypoint is exercised by the
/// shared behavioral suite; this adapter contains no product decisions.
#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedBusiness_evaluateNative(
    mut env: JNIEnv,
    _receiver: JObject,
    input: JString,
) -> jstring {
    let output = catch_unwind(AssertUnwindSafe(|| -> jni::errors::Result<jstring> {
        // The core retains the smaller bound for all non-ledger operations.
        if env.call_method(&input, "length", "()I", &[])?.i()? > 8 * 1024 * 1024 {
            return Ok(env
                .new_string("{\"error\":\"account_response_too_large\"}")?
                .into_raw());
        }
        let input: String = env.get_string(&input)?.into();
        let result = govorun_core::evaluate_business_command(&input);
        Ok(env.new_string(result)?.into_raw())
    }));
    match output {
        Ok(Ok(value)) => value,
        _ => {
            // Preserve an existing VM exception (including allocation failure).
            if !env.exception_check().unwrap_or(true) {
                let _ = env.throw_new(
                    "java/lang/IllegalStateException",
                    "business_bridge_unavailable",
                );
            }
            ptr::null_mut()
        }
    }
}
