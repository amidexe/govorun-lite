package com.govorun.lite.account
import java.time.Instant
import com.govorun.lite.shared.SharedLocalModels
internal fun accountRetryAfterMillis(value: String?, now: Instant = Instant.now()): Long? = SharedLocalModels.retry(value, "account", now)
