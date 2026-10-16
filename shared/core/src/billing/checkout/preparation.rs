use super::validate_preview;
use crate::{AccountRequest, CatalogChannel, Error, Future, HttpResponse, Method, Profile, ProfileUpdate, Result};
use serde_json::{json, Value};

/// Captured authenticated I/O. The adapter supplies neither prices nor checkout decisions.
pub trait CheckoutHost: Send + Sync {
    fn current(&self, epoch: u64) -> Result<()>;
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>>;
    /// Observational publication only. Implementations fence the captured epoch;
    /// failure to render a snapshot never authorizes or replays a payment.
    fn checkout_changed(&self, _snapshot: &super::CheckoutSnapshot) -> Result<()> { Ok(()) }
    fn checkout_account(&self, _epoch: u64) -> Result<String> {
        Err(Error::StorageUnavailable)
    }
    fn begin_checkout_profile(&self, _epoch: u64) -> Result<ProfileUpdate> {
        Err(Error::StorageUnavailable)
    }
    fn publish_checkout_profile(&self, _epoch: u64, _update: ProfileUpdate,
        _profile: Profile) -> Future<'_, Result<()>> {
        Box::pin(async { Err(Error::StorageUnavailable) })
    }
    fn load_checkout(&self, _epoch: u64, _slot: String) -> Future<'_, Result<Option<Value>>> {
        Box::pin(async { Err(Error::StorageUnavailable) })
    }
    fn save_checkout(
        &self,
        _epoch: u64,
        _slot: String,
        _record: Option<Value>,
    ) -> Future<'_, Result<()>> {
        Box::pin(async { Err(Error::StorageUnavailable) })
    }
}

/// Reads current terms and admits the preview as a single session-scoped operation.
/// This operation cannot create a payment or grant access.
pub async fn prepare_checkout(
    host: &dyn CheckoutHost,
    epoch: u64,
    request: Value,
    displayed_catalog: Option<&Value>,
    channel: CatalogChannel,
) -> Result<HttpResponse> {
    host.current(epoch)?;
    validate_request(&request)?;
    let catalog = host
        .request(
            epoch,
            AccountRequest::internal(Method::Get, "/v1/billing/catalog", None),
        )
        .await?;
    host.current(epoch)?;
    if !(200..300).contains(&catalog.status) {
        return Ok(catalog);
    }
    let catalog = crate::catalog_for_channel(catalog.body, channel)?;
    if displayed_catalog.is_some_and(|displayed| !same_published_catalog(displayed, &catalog)) {
        return Ok(HttpResponse {
            status: 409,
            body: json!({"error":"billing_catalog_changed", "catalog":catalog}),
            etag: None,
            retry_after_seconds: None,
        });
    }
    if catalog["channels"]
        .get("tbankPreview")
        .unwrap_or(&catalog["channels"]["tbank"])
        != true
        || !catalog["offers"]
            .as_array()
            .is_some_and(|offers| offers.iter().any(|o| o["offerId"] == request["offerId"]))
    {
        return Err(Error::InvalidRequest);
    }
    let mut response = host
        .request(
            epoch,
            AccountRequest::internal(
                Method::Post,
                "/v1/billing/tbank/preview",
                Some(request.clone()),
            ),
        )
        .await?;
    host.current(epoch)?;
    if (200..300).contains(&response.status) {
        validate_preview(
            &response.body,
            Some((
                &catalog,
                request["offerId"].as_str().ok_or(Error::InvalidRequest)?,
            )),
        )?;
        if let Some(method) = request.get("paymentMethodKind").filter(|v| !v.is_null()) {
            if response
                .body
                .get("paymentMethodKind")
                .unwrap_or(&json!("card"))
                != method
            {
                return Err(Error::InvalidResponse);
            }
        }
        super::preview_action::admit_confirmation(&mut response.body, &catalog);
        response.body = super::project_preview(response.body);
    }
    Ok(response)
}

pub(super) fn validate_request(request: &Value) -> Result<()> {
    let fields = [
        "offerId",
        "promoPreviewId",
        "paymentMethodKind",
        "preferredPaymentMethodKind",
    ];
    if !request
        .as_object()
        .is_some_and(|o| o.keys().all(|k| fields.contains(&k.as_str())))
        || !crate::catalog::identifier(&request["offerId"], 3, true)
    {
        return Err(Error::InvalidRequest);
    }
    for key in ["paymentMethodKind", "preferredPaymentMethodKind"] {
        if request
            .get(key)
            .is_some_and(|v| !v.is_null() && !matches!(v.as_str(), Some("card" | "sbp")))
        {
            return Err(Error::InvalidRequest);
        }
    }
    if request
        .get("promoPreviewId")
        .is_some_and(|v| !v.is_null() && !crate::promotions::valid_preview_id(v))
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

/// Immutable publication signature; volatile server fields cannot affect equality.
#[derive(PartialEq)]
pub(crate) struct CatalogPublication(Value);
impl CatalogPublication {
    fn from_wire(catalog: &Value) -> Self {
        let offers: Vec<Value> = catalog["offers"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|o| {
                let mut result = json!({});
                for key in [
                    "offerId",
                    "planCode",
                    "planName",
                    "billingPeriodLabel",
                    "priceKopeks",
                    "periodDays",
                    "dailyLimit",
                    "periodLimit",
                ] {
                    result[key] = o[key].clone();
                }
                result["ruStoreProductId"] = json!(o["ruStoreProductId"].as_str().unwrap_or(""));
                result["recommended"] = json!(o["recommended"].as_bool().unwrap_or(false));
                result
            })
            .collect();
        Self(json!({"catalogVersion":catalog["catalogVersion"], "currency":catalog["currency"], "offers":offers,
            "storefront":{"state":catalog["storefront"]["state"].as_str().unwrap_or("open"),
                "message":catalog["storefront"]["message"].as_str().unwrap_or("")},
            "programControlVersion":catalog["programControlVersion"].as_str().unwrap_or(""),
            "tbank":catalog["channels"]["tbank"],
            "tbankPreview":catalog["channels"].get("tbankPreview").unwrap_or(&catalog["channels"]["tbank"]),
            "rustore":catalog["channels"]["rustore"].as_bool().unwrap_or(false)}))
    }
}
impl<'de> serde::Deserialize<'de> for CatalogPublication {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        let wire = <Value as serde::Deserialize>::deserialize(decoder)?;
        Ok(Self::from_wire(&wire))
    }
}

pub(crate) fn same_published_catalog(first: &Value, second: &Value) -> bool {
    CatalogPublication::from_wire(first) == CatalogPublication::from_wire(second)
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for CatalogPublication {
    fn schema_name() -> String { "CatalogPublication".into() }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}
