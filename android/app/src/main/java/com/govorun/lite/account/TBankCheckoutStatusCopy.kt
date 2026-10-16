package com.govorun.lite.account

import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

internal val TBankCheckoutSession.presentation
    get() = requireNotNull(clientCheckout) { "Missing shared checkout projection" }

/** The shared owner chooses the message; this adapter only formats local time. */
internal fun tBankCheckoutStatusCopy(
    session: TBankCheckoutSession,
    zoneId: ZoneId = ZoneId.systemDefault(),
): String {
    val message = session.presentation.message
    if (!message.contains("{firstChargeDate}")) return message
    val moment = Instant.parse(session.firstChargeAt).atZone(zoneId)
    return message.replace("{firstChargeDate}", moment.format(DateTimeFormatter.ofPattern("d MMMM", Locale.forLanguageTag("ru"))))
        .replace("{firstChargeTime}", moment.format(DateTimeFormatter.ofPattern("HH:mm", Locale.ROOT)))
}
