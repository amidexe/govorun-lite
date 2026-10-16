package com.govorun.lite.account

internal fun publishedAccountState(
    apiConfigured: Boolean,
    session: GovorunSession,
    syncing: Boolean,
    devices: List<AccountDevice>,
) = AccountState(
    available = apiConfigured,
    signedIn = true,
    accountId = session.accountId.orEmpty(),
    displayName = session.displayName,
    onlineTermsVersion = session.onlineTermsVersion,
    syncing = syncing,
    devices = devices,
    deviceLimit = session.deviceLimit,
    styleAccess = session.styleAccess,
    starterGrant = session.starterGrant,
    promoProgram = session.promoProgram,
    freeAccessAvailable = session.freeAccessAvailable,
    onlineConsent = session.onlineConsent,
    onlineService = session.onlineService,
)

/** Ancillary sync cannot replace a live profile with a persisted, untrusted clock. */
internal fun publishedAccountSyncMetadata(
    current: AccountState,
    syncing: Boolean,
    devices: List<AccountDevice>,
): AccountState = current.copy(syncing = syncing, devices = devices)
