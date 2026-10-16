package com.govorun.lite.account

import kotlinx.coroutines.CancellationException

/** A confirmed checkout needs one status read, independent of catalog/profile refresh. */
internal suspend fun loadConfirmedCheckoutStatus(load: suspend () -> BillingStatus): BillingStatus? =
    try {
        load()
    } catch (error: CancellationException) {
        throw error
    } catch (_: Exception) {
        null
    }
