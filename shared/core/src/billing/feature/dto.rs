use crate::features::dto::dto;
dto!(PromoProgram {
    state: String, visible: bool, can_activate: bool, program_version: String,
    participated: bool, enrollment_open: bool, access_mode: String, access_days: u64,
    ends_at: Option<String>, daily_limit: u64, period_limit: u64, period_days: u64, message: String
});
dto!(PromoPreview { id: String, title: String, body: String, checkout_required: bool, bonus_days: Option<u64> });
dto!(BillingPromotionAccess { id: String, kind: crate::PromotionAccessKind, terms_version: Option<u32>, message: String, deferred_message: String });
dto!(BillingBalance { title: String, value: String, detail: Option<String>, percent: Option<u64> });
dto!(BillingPlan { id: String, title: String, multiplier: Option<String>, price: String, period: String, limit: String, trial: String, current: bool });
dto!(BillingStorefront {
    state: String,
    message: String
});
dto!(BillingStatus {
    state: String, fresh: bool, loading: bool, payment: String, title: String,
    period_end: Option<String>, renewal_notice: Option<String>, checkout_expires_at: Option<String>,
    card: Option<String>, binding_url: Option<String>, payment_method_unavailable: bool,
    trial_active: bool, pending_checkout: bool, can_resume_checkout: bool, can_restart_binding: bool,
    can_cancel_renewal: bool, can_cancel_change: bool, can_remove_card: bool
});
dto!(BillingCheckout {
    id: String, title: String, plan_title: String, message: Option<String>, payment_method_kind: String,
    available_payment_methods: Vec<String>, trial_headline: String, amount_due_now: String,
    first_charge: String, renewal: String, handoff: String, email_masked: Option<String>, email: Option<String>,
    email_required: bool, receipt_contact_needed: bool, receipt_purpose: String, action_title: String,
    recurring_disclosure: String, consent: String, action_enabled: bool, has_legal_documents: bool
});
dto!(BillingSnapshot {
    scope: String, signed_in: bool, busy: bool, message: Option<String>, plan_title: String,
    has_access: bool, access_type: String, promo_program: Option<PromoProgram>, storefront: BillingStorefront,
    show_plan_intro: bool, show_plans: bool, show_payment_details: bool,
    refresh_after_seconds: u64, balances: Vec<BillingBalance>, period: Option<String>,
    can_activate_starter: bool, terms_version: u32, status: BillingStatus, catalog_state: String,
    plans: Vec<BillingPlan>, plan_action: String, plan_disclosure: Option<String>,
    can_preview_checkout: bool, checkout: Option<BillingCheckout>, promo: Option<PromoPreview>, promo_access: Option<BillingPromotionAccess>
});
