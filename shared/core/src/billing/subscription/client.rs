use crate::{CatalogChannel, Error, Result};
use serde::Deserialize;
use serde_json::Value;

/// Native presentation capabilities, not alternative purchase rules.
#[derive(Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Client {
    #[serde(default)]
    channel: Option<CatalogChannel>,
    #[serde(default)]
    bank_presentation: BankPresentation,
    preferred_payment_method: Option<String>,
}

#[derive(Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum BankPresentation { Browser, #[default] InlineSbp }

impl Client {
    pub fn read(command: &Value) -> Result<Self> {
        let value = match command.get("client") {
            None => Self::default(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|_| Error::InvalidRequest)?,
        };
        if value.preferred_payment_method.as_deref().is_some_and(|method| !matches!(method, "card" | "sbp")) {
            return Err(Error::InvalidRequest);
        }
        Ok(value)
    }
    pub fn channel(&self) -> CatalogChannel { self.channel.unwrap_or(CatalogChannel::Direct) }
    pub fn opens_sbp(&self) -> bool { matches!(self.bank_presentation, BankPresentation::Browser) }
    pub fn preferred_method(&self) -> Option<&str> { self.preferred_payment_method.as_deref() }
}
