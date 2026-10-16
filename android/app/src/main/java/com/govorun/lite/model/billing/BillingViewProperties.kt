package com.govorun.lite.model.billing

val BillingStorefront.unavailable: Boolean get() = state == "unavailable"
val BillingStorefront.unavailableMessage: String get() = message.ifBlank { "В данный момент недоступно" }
