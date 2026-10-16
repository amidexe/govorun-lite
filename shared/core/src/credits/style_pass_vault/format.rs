//! Current protected-store layouts, not a legacy migration path. Dates and byte
//! encoding are properties of the stored format; product rules remain common.
use crate::{style_pass_crypto::RequestSnapshot, Error, Result};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Format {
    LedgerV5,
    FlatV4,
    AndroidV8,
}
impl Format {
    pub fn date_millis(self, value: &Value) -> Result<u64> {
        let millis = match self {
            Self::LedgerV5 => {
                let seconds = value
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or(Error::StorageInvalid)?;
                let millis = (seconds + 978_307_200.0) * 1000.0;
                if !(0.0..=253_402_300_799_999.0).contains(&millis) {
                    return Err(Error::StorageInvalid);
                }
                millis.trunc() as u64
            }
            Self::AndroidV8 => value.as_u64().and_then(|v| v.checked_mul(1000)).ok_or(Error::StorageInvalid)?,
            Self::FlatV4 => {
                let value = value.as_str().ok_or(Error::StorageInvalid)?;
                let time =
                    DateTime::parse_from_rfc3339(value).map_err(|_| Error::StorageInvalid)?;
                u64::try_from(time.timestamp_millis()).map_err(|_| Error::StorageInvalid)?
            }
        };
        if millis == 0 || millis > 253_402_300_799_999 {
            return Err(Error::StorageInvalid);
        }
        Ok(millis)
    }
    pub fn date_value(self, millis: u64) -> Result<Value> {
        if millis == 0 || millis > 253_402_300_799_999 {
            return Err(Error::StorageInvalid);
        }
        match self {
            Self::LedgerV5 => Ok(serde_json::json!(millis as f64 / 1000.0 - 978_307_200.0)),
            Self::AndroidV8 => Ok(serde_json::json!(millis / 1000)),
            Self::FlatV4 => DateTime::from_timestamp_millis(millis as i64)
                .map(|d| serde_json::json!(d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)))
                .ok_or(Error::StorageInvalid),
        }
    }
    pub fn key_id_field(self) -> &'static str {
        if self == Self::LedgerV5 {
            "keyID"
        } else {
            "keyId"
        }
    }
    pub(super) fn journal_id_field(self) -> &'static str {
        if self == Self::AndroidV8 { "id" } else { "idempotencyKey" }
    }
    fn decode_bytes(self, value: &Value, max: usize) -> Result<String> {
        let s = value
            .as_str()
            .filter(|s| s.len() <= max.div_ceil(3) * 4)
            .ok_or(Error::StorageInvalid)?;
        let bytes = if self == Self::LedgerV5 {
            STANDARD.decode(s)
        } else {
            URL_SAFE_NO_PAD.decode(s)
        }
        .map_err(|_| Error::StorageInvalid)?;
        if bytes.is_empty() || bytes.len() > max {
            return Err(Error::StorageInvalid);
        }
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }
    pub fn request(self, raw: &Value) -> Result<RequestSnapshot> {
        if self == Self::AndroidV8 { return Err(Error::StorageInvalid); }
        let key = self.key_id_field();
        fields(
            raw,
            &[
                key,
                "preparedMessage",
                "blindedMessage",
                "encodedMessage",
                "inverse",
                "modulus",
                "exponent",
                "keyNotBefore",
                "keyNotAfter",
            ],
            &[],
        )?;
        let request = RequestSnapshot {
            key_id: text(raw, key, 64)?,
            prepared_message: self.decode_bytes(&raw["preparedMessage"], 256)?,
            blinded_message: self.decode_bytes(&raw["blindedMessage"], 1024)?,
            encoded_message: self.decode_bytes(&raw["encodedMessage"], 1024)?,
            inverse: self.decode_bytes(&raw["inverse"], 1024)?,
            modulus: self.decode_bytes(&raw["modulus"], 1024)?,
            exponent: self.decode_bytes(&raw["exponent"], 16)?,
            key_not_before_millis: self.date_millis(&raw["keyNotBefore"])?,
            key_not_after_millis: self.date_millis(&raw["keyNotAfter"])?,
        };
        crate::style_pass_crypto::BlindRequest::restore(request.clone())
            .map_err(|_| Error::StorageInvalid)?;
        Ok(request)
    }
    pub fn stored_request(self, request: &RequestSnapshot) -> Result<Value> {
        if self == Self::AndroidV8 { return Err(Error::StorageInvalid); }
        crate::style_pass_crypto::BlindRequest::restore(request.clone())
            .map_err(|_| Error::StorageInvalid)?;
        let mut value = serde_json::Map::new();
        value.insert(
            self.key_id_field().into(),
            Value::String(request.key_id.clone()),
        );
        for (name, s) in [
            ("preparedMessage", &request.prepared_message),
            ("blindedMessage", &request.blinded_message),
            ("encodedMessage", &request.encoded_message),
            ("inverse", &request.inverse),
            ("modulus", &request.modulus),
            ("exponent", &request.exponent),
        ] {
            let b = URL_SAFE_NO_PAD
                .decode(s)
                .map_err(|_| Error::StorageInvalid)?;
            value.insert(
                name.into(),
                Value::String(if self == Self::LedgerV5 {
                    STANDARD.encode(b)
                } else {
                    s.clone()
                }),
            );
        }
        value.insert(
            "keyNotBefore".into(),
            self.date_value(request.key_not_before_millis)?,
        );
        value.insert(
            "keyNotAfter".into(),
            self.date_value(request.key_not_after_millis)?,
        );
        Ok(Value::Object(value))
    }
}
pub(super) fn fields(value: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let map = value.as_object().ok_or(Error::StorageInvalid)?;
    if required.iter().any(|k| !map.contains_key(*k))
        || map
            .keys()
            .any(|k| !required.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err(Error::StorageInvalid);
    }
    Ok(())
}
pub(super) fn text(value: &Value, key: &str, max: usize) -> Result<String> {
    value[key]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= max)
        .map(str::to_owned)
        .ok_or(Error::StorageInvalid)
}
