package com.govorun.lite.account

/** Profile refreshes are allowed; signing out and signing in starts a new session. */
internal fun sameActivationAccount(before: AccountState, current: AccountState): Boolean =
    before.signedIn && current.signedIn && before.accountId.isNotBlank() &&
        before.accountId == current.accountId &&
        before.sessionGeneration == current.sessionGeneration &&
        current.secureStorageProblem == null

/** Promo replaces Starter while visible; eligibility never grants access by itself. */
internal fun canActivateFreeStyleAccess(state: AccountState): Boolean = state.signedIn && state.freeAccessAvailable

internal fun styleSetupNeedsSubscription(state: AccountState): Boolean =
    state.styleWindowPresentation().setupNeedsSubscription

internal fun starterStyleLimitExhausted(state: AccountState): Boolean =
    state.styleWindowPresentation().starterLimitExhausted
