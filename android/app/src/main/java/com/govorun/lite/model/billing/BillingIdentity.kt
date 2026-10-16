package com.govorun.lite.model.billing

data class BillingIdentity(
    val signedIn: Boolean,
    val generation: Long,
    val accountId: String = "",
)
