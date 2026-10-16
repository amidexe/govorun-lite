// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BillingStatus(
    val `bindingUrl`: String?,
    val `canCancelChange`: Boolean,
    val `canCancelRenewal`: Boolean,
    val `canRemoveCard`: Boolean,
    val `canRestartBinding`: Boolean,
    val `canResumeCheckout`: Boolean,
    val `card`: String?,
    val `checkoutExpiresAt`: String?,
    val `fresh`: Boolean,
    val `loading`: Boolean,
    val `payment`: String,
    val `paymentMethodUnavailable`: Boolean,
    val `pendingCheckout`: Boolean,
    val `periodEnd`: String?,
    val `renewalNotice`: String?,
    val `state`: String,
    val `title`: String,
    val `trialActive`: Boolean
)
