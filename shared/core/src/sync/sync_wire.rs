//! Current sync envelope admission, shared by every client transport binding.
use crate::{content_sync, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ContentResponse {
    pub not_modified: bool,
    pub configured: bool,
    pub etag: String,
    pub document: Value,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct Envelope {
    schema_version: u32,
    configured: bool,
    document: content_sync::Document,
}

/// Successful reads retain the prior ETag fallback; writes require a new ETag.
/// An error status is not mistaken for an empty document or an acknowledgement.
pub fn content_response(
    response: &Value,
    previous_etag: Option<&str>,
    writing: bool,
) -> Result<ContentResponse> {
    let status = response["status"]
        .as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or(Error::InvalidResponse)?;
    if status != 200 && !(status == 304 && !writing) {
        return Err(Error::Http(status));
    }
    let etag = response["etag"]
        .as_str()
        .or(if writing { None } else { previous_etag })
        .filter(|value| {
            !value.trim().is_empty()
                && value.len() <= 256
                && !value.bytes().any(|b| b.is_ascii_control())
        })
        .ok_or(Error::InvalidResponse)?
        .to_owned();
    if status == 304 {
        return Ok(ContentResponse {
            not_modified: true,
            configured: false,
            etag,
            document: serde_json::to_value(content_sync::Document::default())
                .map_err(|_| Error::InvalidResponse)?,
        });
    }
    let envelope: Envelope =
        serde_json::from_value(response["body"].clone()).map_err(|_| Error::InvalidResponse)?;
    if envelope.schema_version != 1 {
        return Err(Error::InvalidResponse);
    }
    content_sync::validate(&envelope.document).map_err(|_| Error::InvalidResponse)?;
    Ok(ContentResponse {
        not_modified: false,
        configured: writing || envelope.configured,
        etag,
        document: serde_json::to_value(envelope.document).map_err(|_| Error::InvalidResponse)?,
    })
}
