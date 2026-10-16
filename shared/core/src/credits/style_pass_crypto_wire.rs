//! Private crypto exchange for host/JNI callers. Never a renderer route.
//! Batch operations are atomic: one invalid item yields no partial token result.
use super::style_pass_crypto::{
    CryptoError, RequestSnapshot, RustStylePassCrypto, StylePassCryptoPort, StylePassKey,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Key {
    pub key_id: String,
    pub algorithm: String,
    pub public_key: String,
    pub not_before_millis: u64,
    pub not_after_millis: u64,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    InspectKey { key: Key },
    PrepareAndroid { key: Key, count: usize, expires_at_millis: u64, now_millis: u64 },
    RestoreAndroid { key: Key, requests: Vec<super::style_pass_crypto::android_material::Material> },
    FinalizeAndroid { key: Key, requests: Vec<super::style_pass_crypto::android_material::Material>, signatures: Vec<String> },
    Prepare {
        key: Key,
        count: usize,
        expires_at_millis: u64,
        now_millis: u64,
    },
    Restore {
        requests: Vec<RequestSnapshot>,
    },
    Finalize {
        requests: Vec<RequestSnapshot>,
        signatures: Vec<String>,
    },
}

pub fn execute(command: Command) -> Result<Value, CryptoError> {
    let port = RustStylePassCrypto;
    match command {
        Command::InspectKey { key } => { parse_key(key)?; Ok(json!({"valid":true})) }
        Command::PrepareAndroid { key, count, expires_at_millis, now_millis } => {
            count_valid(count)?;
            let key = parse_key(key)?;
            let requests = (0..count).map(|_| {
                super::style_pass_crypto::android_material::prepare(&key, expires_at_millis, now_millis)
                    .map(|(r, state)| json!({"state":state, "blindedMessage":B64.encode(r.blinded_message())}))
            }).collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"requests":requests}))
        }
        Command::RestoreAndroid { key, requests } => {
            let requests = android_requests(key, requests)?;
            Ok(json!({"blindedMessages":requests.iter().map(|r| B64.encode(r.blinded_message())).collect::<Vec<_>>()}))
        }
        Command::FinalizeAndroid { key, requests, signatures } => {
            if requests.len() != signatures.len() { return Err(CryptoError::InvalidRequest); }
            let tokens = android_requests(key, requests)?.iter().zip(signatures)
                .map(|(r,s)| r.finalize(&decode(&s,1024)?)).collect::<Result<Vec<_>,_>>()?;
            Ok(json!({"tokens":tokens}))
        }
        Command::Prepare {
            key,
            count,
            expires_at_millis,
            now_millis,
        } => {
            count_valid(count)?;
            let key = parse_key(key)?;
            let requests = (0..count)
                .map(|_| {
                    port.prepare(&key, expires_at_millis, now_millis)
                        .map(|r| r.snapshot())
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"requests":requests}))
        }
        Command::Restore { requests } => {
            same_key(&requests)?;
            let messages = requests
                .into_iter()
                .map(|r| port.restore(r).map(|r| B64.encode(r.blinded_message())))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"blindedMessages":messages}))
        }
        Command::Finalize {
            requests,
            signatures,
        } => {
            same_key(&requests)?;
            if requests.len() != signatures.len() {
                return Err(CryptoError::InvalidRequest);
            }
            let tokens = requests
                .into_iter()
                .zip(signatures)
                .map(|(r, s)| port.restore(r)?.finalize(&decode(&s, 1024)?))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"tokens":tokens}))
        }
    }
}
fn count_valid(count: usize) -> Result<(), CryptoError> {
    if !(1..=crate::style_inventory::BATCH).contains(&count) {
        return Err(CryptoError::InvalidRequest);
    }
    Ok(())
}
fn same_key(requests: &[RequestSnapshot]) -> Result<(), CryptoError> {
    count_valid(requests.len())?;
    let first = &requests[0];
    if requests.iter().any(|r| {
        r.key_id != first.key_id
            || r.modulus != first.modulus
            || r.exponent != first.exponent
            || r.key_not_before_millis != first.key_not_before_millis
            || r.key_not_after_millis != first.key_not_after_millis
    }) {
        return Err(CryptoError::InvalidRequest);
    }
    Ok(())
}
fn decode(value: &str, max: usize) -> Result<Vec<u8>, CryptoError> {
    if value.len() > max.div_ceil(3) * 4 {
        return Err(CryptoError::InvalidRequest);
    }
    let bytes = B64.decode(value).map_err(|_| CryptoError::InvalidRequest)?;
    if bytes.is_empty() || bytes.len() > max || B64.encode(&bytes) != value {
        return Err(CryptoError::InvalidRequest);
    }
    Ok(bytes)
}

fn parse_key(key: Key) -> Result<StylePassKey, CryptoError> {
    StylePassKey::from_spki(&key.key_id, &key.algorithm, &decode(&key.public_key, 2048)?,
        key.not_before_millis, key.not_after_millis)
}
fn android_requests(key: Key, requests: Vec<super::style_pass_crypto::android_material::Material>)
    -> Result<Vec<super::style_pass_crypto::BlindRequest>, CryptoError> {
    count_valid(requests.len())?;
    let key = parse_key(key)?;
    requests.into_iter().map(|r| super::style_pass_crypto::android_material::restore(&key, r)).collect()
}
