//! Preserve domain validation errors when decoding untrusted server documents.
//! Admission belongs to each existing rule owner; transport decoding cannot
//! turn a malformed server disclosure into a different user-command error.
use crate::Result;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub(super) trait ServerModel: Sized {
    fn admit(wire: Value) -> Result<Self>;
}
pub(super) struct ServerInput<T: ServerModel>(Result<T>);
impl<T: ServerModel> ServerInput<T> {
    pub(super) fn resolve(self) -> Result<T> { self.0 }
}
impl<'de, T: ServerModel> Deserialize<'de> for ServerInput<T> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        Ok(Self(T::admit(Value::deserialize(decoder)?)))
    }
}

/// The immutable server catalog admitted by the sole catalog rule owner.
/// Preserve server extensions and absent/null distinctions in financial data.
pub(super) struct Catalog(Value);
impl ServerModel for Catalog {
    fn admit(wire: Value) -> Result<Self> {
        crate::validate_catalog(&wire)?;
        Ok(Self(wire))
    }
}
impl Catalog {
    pub(super) fn project(self, channel: crate::CatalogChannel) -> Result<Value> {
        crate::catalog_for_channel(self.0, channel)
    }
}

/// Only a successfully inspected offer can reach the final catalog admission.
pub(super) struct PromotionOffer { wire: Value, requires_catalog: bool }
impl ServerModel for PromotionOffer {
    fn admit(wire: Value) -> Result<Self> {
        let requires_catalog = crate::promotions::inspect_offer(&wire)?;
        Ok(Self { wire, requires_catalog })
    }
}
impl PromotionOffer {
    pub(super) fn requires_catalog(&self) -> bool { self.requires_catalog }
    pub(super) fn validate(&self, catalog: Option<&PromotionCatalog>, requested: Option<&str>) -> Result<()> {
        crate::promotions::validate_offer(&self.wire, catalog.map(PromotionCatalog::wire), requested)
    }
}

/// A promotion compares only catalog publication fields. It deliberately is
/// not a full catalog admission: the existing owner permits partial contexts.
pub(super) struct PromotionCatalog(Value);
impl<'de> Deserialize<'de> for PromotionCatalog {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        let wire = Value::deserialize(decoder)?;
        Ok(Self(serde_json::json!({"offers":wire["offers"],
            "catalogVersion":wire["catalogVersion"], "currency":wire["currency"]})))
    }
}
impl PromotionCatalog {
    fn wire(&self) -> &Value { &self.0 }
}

/// A recovery decision computed from a validated checkout session.
pub(super) struct CheckoutSession(Value);
impl ServerModel for CheckoutSession {
    fn admit(wire: Value) -> Result<Self> {
        crate::checkout::projection(&wire).map(Self)
    }
}
impl CheckoutSession {
    pub(super) fn projection(self) -> Value { self.0 }
}

/// A validated disclosure; decoding cannot create or confirm a purchase.
pub(super) struct CheckoutPreview(Value);
impl ServerModel for CheckoutPreview {
    fn admit(wire: Value) -> Result<Self> {
        crate::validate_checkout_preview(&wire, None)?;
        Ok(Self(crate::checkout::project_preview(wire)))
    }
}
impl CheckoutPreview {
    pub(super) fn projection(self) -> Value { self.0 }
}

impl ServerModel for crate::Profile {
    fn admit(wire: Value) -> Result<Self> { Self::try_from(wire) }
}


#[cfg(feature = "schema")]
impl<T: ServerModel> schemars::JsonSchema for ServerInput<T> {
    fn schema_name() -> String { format!("ServerInput_{}", std::any::type_name::<T>().rsplit("::").next().unwrap()) }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Decoding retains the domain Result for every JSON shape. Semantic admission
        // is authoritative; invalid server input returns its original domain error.
        schemars::schema::Schema::Bool(true)
    }
}
#[cfg(feature = "schema")]
impl schemars::JsonSchema for PromotionCatalog {
    fn schema_name() -> String { "PromotionCatalog".into() }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}
