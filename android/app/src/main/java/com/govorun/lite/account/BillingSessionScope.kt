package com.govorun.lite.account

/**
 * Server billing state belongs to the exact account session that requested it.
 * A response from a signed-out or replaced account must never become current UI state.
 */
internal typealias BillingSessionScope = com.govorun.lite.model.billing.BillingIdentity

internal fun billingSessionScope(state: AccountState): BillingSessionScope = BillingSessionScope(
    signedIn = state.signedIn,
    generation = state.sessionGeneration,
    accountId = state.accountId,
)
