use crate::{AccountRequest, AccountSession, Error, Future, HttpResponse, Method, Result};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

/// A captured account/public transport. No fallback to a different identity is allowed.
pub trait CatalogSource: Send + Sync {
    fn current(&self, epoch: u64) -> Result<()>;
    fn fetch(&self, epoch: u64) -> Future<'_, Result<HttpResponse>>;
    fn channel(&self) -> CatalogChannel {
        CatalogChannel::Direct
    }
    fn catalog_changed(&self, _snapshot: &crate::CatalogSnapshot) -> Result<()> {
        Ok(())
    }
}

/// Every client loads and validates a fresh server catalog through this owner.
/// Errors never return a cached catalog or masquerade as an explicitly closed storefront.
pub async fn load_catalog(source: &dyn CatalogSource, epoch: u64) -> Result<HttpResponse> {
    source.current(epoch)?;
    let response = source.fetch(epoch).await;
    source.current(epoch)?;
    let mut response = response?;
    if (200..300).contains(&response.status) {
        validate_catalog(&response.body)?;
        response.body = crate::access_offer::project_catalog(response.body);
    }
    Ok(response)
}

#[derive(Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum CatalogChannel {
    Direct,
}

pub fn catalog_for_channel(catalog: Value, _channel: CatalogChannel) -> Result<Value> {
    validate_catalog(&catalog)?;
    Ok(crate::access_offer::project_catalog(catalog))
}

pub fn validate_catalog(catalog: &Value) -> Result<()> {
    let offers = catalog["offers"].as_array().ok_or(Error::InvalidResponse)?;
    let channels = &catalog["channels"];
    let trial = &catalog["trialOffer"];
    if catalog["schemaVersion"] != 1
        || !identifier(&catalog["catalogVersion"], 1, false)
        || catalog["currency"] != "RUB"
        || !integer(&catalog["refreshAfterSeconds"], 60, 3600)
        || catalog["publishedAt"]
            .as_str()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .is_none()
        || !channels.is_object()
        || !channels["tbank"].is_boolean()
        || !optional_bool(channels, "tbankPreview")
        || !optional_bool(channels, "rustore")
        || !trial.is_object()
        || !text(&trial["state"])
        || !trial["eligible"].is_boolean()
        || trial["requiresVerifiedPaymentMethod"] != true
        || trial["checkoutOpenGrantsAccess"] != false
        || (trial["state"] == "available"
            && trial["eligible"] == true
            && !integer(&trial["durationDays"], 1, 366))
    {
        return Err(Error::InvalidResponse);
    }
    if let Some(storefront) = catalog.get("storefront").filter(|v| !v.is_null()) {
        if !matches!(storefront["state"].as_str(), Some("open" | "unavailable"))
            || storefront.get("message").is_some_and(|v| !v.is_string())
        {
            return Err(Error::InvalidResponse);
        }
    }
    if closed(catalog) {
        if !offers.is_empty()
            || channels["tbank"] == true
            || channels["tbankPreview"] == true
            || channels["rustore"] == true
        {
            return Err(Error::InvalidResponse);
        }
    } else if !(2..=32).contains(&offers.len()) || !both_plans(offers) {
        return Err(Error::InvalidResponse);
    }
    let mut ids = HashSet::new();
    for offer in offers {
        if !identifier(&offer["offerId"], 3, true)
            || !ids.insert(&offer["offerId"])
            || !matches!(offer["planCode"].as_str(), Some("base" | "pro"))
            || !text(&offer["planName"])
            || !text(&offer["billingPeriodLabel"])
            || !integer(&offer["priceKopeks"], 1, 100_000_000)
            || !integer(&offer["periodDays"], 1, 366)
            || !integer(&offer["dailyLimit"], 1, i32::MAX as u64)
            || !integer(
                &offer["periodLimit"],
                offer["dailyLimit"].as_u64().unwrap_or(u64::MAX),
                i32::MAX as u64,
            )
        {
            return Err(Error::InvalidResponse);
        }
    }
    Ok(())
}

fn closed(catalog: &Value) -> bool {
    catalog["storefront"]["state"] == "unavailable"
}
fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|v| !v.trim().is_empty())
}
fn integer(value: &Value, min: u64, max: u64) -> bool {
    value.as_u64().is_some_and(|n| n >= min && n <= max)
}
fn optional_bool(value: &Value, key: &str) -> bool {
    value.get(key).is_none_or(|v| v.is_null() || v.is_boolean())
}
fn both_plans(offers: &[Value]) -> bool {
    ["base", "pro"]
        .iter()
        .all(|code| offers.iter().any(|offer| offer["planCode"] == *code))
}
pub(crate) fn identifier(value: &Value, min: usize, lower: bool) -> bool {
    value.as_str().is_some_and(|v| {
        (min..=64).contains(&v.len())
            && v.bytes()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() && (!lower || !c.is_ascii_uppercase()))
            && v.bytes().all(|c| {
                (c.is_ascii_alphanumeric() && (!lower || !c.is_ascii_uppercase()))
                    || b"._-".contains(&c)
            })
    })
}

struct SessionCatalog<'a> {
    session: &'a AccountSession,
    public: bool,
}
impl CatalogSource for SessionCatalog<'_> {
    fn catalog_changed(&self, snapshot: &crate::CatalogSnapshot) -> Result<()> {
        self.session.current(snapshot.epoch)?;
        if snapshot.state != "loading" {
            self.session.inner.subscription.catalog_changed(snapshot.epoch,
                if snapshot.state == "loaded" { snapshot.catalog.as_ref() } else { None });
        }
        drop(self.session.mutate());
        self.session.changed();
        Ok(())
    }
    fn current(&self, epoch: u64) -> Result<()> {
        self.session.current(epoch)
    }
    fn fetch(&self, epoch: u64) -> Future<'_, Result<HttpResponse>> {
        Box::pin(async move {
            let request = AccountRequest::internal(Method::Get, "/v1/billing/catalog", None);
            if self.public {
                self.session
                    .send_scoped(
                        epoch,
                        self.session.inner.boundary.subscribe(),
                        request,
                        None,
                    )
                    .await
            } else {
                self.session.request_at(epoch, request).await
            }
        })
    }
}

impl AccountSession {
    pub async fn refresh_catalog(&self, epoch: u64) -> Result<()> {
        self.refresh_catalog_response(epoch).await?.success().map(|_| ())
    }

    pub(crate) async fn refresh_catalog_response(&self, epoch: u64) -> Result<HttpResponse> {
        let identity = self.snapshot();
        if identity.session_generation != epoch { return Err(Error::SessionChanged); }
        self.inner.catalog.fetch(&SessionCatalog {
            session: self, public: !identity.signed_in,
        }, epoch).await
    }
    pub async fn public_catalog(&self) -> Result<HttpResponse> {
        load_catalog(
            &SessionCatalog {
                session: self,
                public: true,
            },
            self.snapshot().session_generation,
        )
        .await
    }
    pub(crate) async fn catalog_at(&self, epoch: u64) -> Result<HttpResponse> {
        load_catalog(
            &SessionCatalog {
                session: self,
                public: false,
            },
            epoch,
        )
        .await
    }
}
