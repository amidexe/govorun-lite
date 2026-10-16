use crate::{Authority, Profile, TermsRelation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PromotionAccessKind {
    Done,
    AcceptTerms,
    Refresh,
}

/// A confirmed redemption and its next UI effect. It never grants local permission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PromotionAccess {
    pub kind: PromotionAccessKind,
    pub terms_version: Option<u32>,
    pub message: &'static str,
    pub deferred_message: &'static str,
}

impl PromotionAccess {
    pub(crate) fn checking() -> Self {
        Self {
            kind: PromotionAccessKind::Refresh,
            terms_version: None,
            message: "Промокод применён. Не удалось проверить доступ. Обновите сведения.",
            deferred_message: "Промокод применён. Проверить доступ можно позже.",
        }
    }

    pub(crate) fn confirmed(profile: &Profile, terms: u32) -> Self {
        let authority = Authority::evaluate(Some(profile), terms, true, true);
        let needs_consent = authority.entitlement_active
            && authority.protocol_supported
            && authority.terms_relation == TermsRelation::Match
            && !authority.consent_accepted;
        Self {
            kind: if needs_consent {
                PromotionAccessKind::AcceptTerms
            } else {
                PromotionAccessKind::Done
            },
            terms_version: needs_consent.then_some(terms),
            message: "Промокод применён.",
            deferred_message: "Промокод применён. Улучшение текста можно включить позже.",
        }
    }
}
