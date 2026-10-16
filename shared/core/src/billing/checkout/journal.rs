use super::{
    confirmation::confirmation, preparation::validate_request, session::matches_confirmation,
    validate_preview,
};
use crate::{AccountRequest, Error, HttpResponse, Method, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Protected, account-scoped write-ahead record. Never a renderer view or diagnostic.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Journal {
    version: u32,
    account_id: String,
    pub selection: Value,
    pub preview: Value,
    path: String,
    body: Value,
    response: Option<SavedResponse>,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct SavedResponse {
    status: u16,
    body: Value,
}

pub(super) fn slot(account: &str) -> Result<String> {
    if account.is_empty() || account.len() > 128 || account.chars().any(char::is_control) {
        return Err(Error::StorageInvalid);
    }
    let hash = Sha256::digest(account.as_bytes());
    Ok(format!(
        "account-purchase-v1-{}",
        hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

impl Journal {
    pub fn encode(
        account: &str,
        selection: &Value,
        preview: &Value,
        request: &AccountRequest,
        response: Option<&HttpResponse>,
    ) -> Result<Value> {
        let value = serde_json::to_value(Self {
            version: 1,
            account_id: account.into(),
            selection: selection.clone(),
            preview: preview.clone(),
            path: request.path().into(),
            body: request.body().cloned().ok_or(Error::StorageInvalid)?,
            response: response.map(|r| SavedResponse {
                status: r.status,
                body: r.body.clone(),
            }),
        })
        .map_err(|_| Error::StorageInvalid)?;
        if value.to_string().len() > 65_536 {
            return Err(Error::StorageInvalid);
        }
        Ok(value)
    }

    pub fn decode(account: &str, value: Value) -> Result<Self> {
        if value.to_string().len() > 65_536 {
            return Err(Error::StorageInvalid);
        }
        let result = (|| -> Result<Self> {
            let record: Self = serde_json::from_value(value).map_err(|_| Error::StorageInvalid)?;
            if record.version != 1 || record.account_id != account || record.preview["checkoutAvailable"] != true {
                return Err(Error::StorageInvalid);
            }
            validate_preview(&record.preview, None)?;
            validate_request(&record.selection)?;
            if record
                .selection
                .get("paymentMethodKind")
                .is_some_and(|method| {
                    !method.is_null()
                        && method
                            != record
                                .preview
                                .get("paymentMethodKind")
                                .unwrap_or(&serde_json::json!("card"))
                })
            {
                return Err(Error::StorageInvalid);
            }
            if record.selection["offerId"] != record.preview["offerId"]
                || !crate::promotions::valid_preview_id(&record.body["idempotencyKey"])
            {
                return Err(Error::StorageInvalid);
            }
            let input = record.intent()?;
            if confirmation(&record.preview, &record.path, &input)? != input {
                return Err(Error::StorageInvalid);
            }
            if let Some(reply) = &record.response {
                if (200..300).contains(&reply.status) {
                    matches_confirmation(&reply.body, &record.preview)?;
                } else if !matches!(reply.status, 400 | 409) {
                    return Err(Error::StorageInvalid);
                }
            }
            Ok(record)
        })();
        result.map_err(|_| Error::StorageInvalid)
    }

    pub fn intent(&self) -> Result<Value> {
        let mut body = self.body.clone();
        body.as_object_mut()
            .ok_or(Error::StorageInvalid)?
            .remove("idempotencyKey");
        Ok(body)
    }
    pub fn request(&self) -> AccountRequest {
        AccountRequest::internal(Method::Post, &self.path, Some(self.body.clone()))
    }
    pub fn response(&self) -> Option<HttpResponse> {
        self.response.as_ref().map(|r| HttpResponse {
            status: r.status,
            body: r.body.clone(),
            etag: None,
            retry_after_seconds: None,
        })
    }
}
