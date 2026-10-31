package com.govorun.lite.account

import org.json.JSONObject
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** Decoded shared card; date formatting belongs to the native UI. */
internal class StyleQuotaPresentation(private val wire: JSONObject) {
    private val equalityKey = wire.toString()
    override fun equals(other: Any?): Boolean =
        other is StyleQuotaPresentation && equalityKey == other.equalityKey
    override fun hashCode(): Int = equalityKey.hashCode()

    val kind: String get() = wire.getString("kind")
    val planTitle: String get() = wire.getString("planTitle")
    val heading: String get() = wire.getString("heading")
    val refreshing: Boolean get() = wire.getBoolean("refreshing")
    val todayPercent: Int? get() = percent("todayPercent")
    val periodPercent: Int? get() = percent("periodPercent")

    fun periodRange(zone: ZoneId, locale: Locale): String? {
        val start = date("periodStart", zone, locale) ?: return null
        val end = date("periodEnd", zone, locale) ?: return null
        return "Период: $start — $end"
    }

    fun todayReset(zone: ZoneId, locale: Locale): String? =
        date("todayReset", zone, locale, time = true)?.let { "Обновится $it" }

    fun periodStatus(zone: ZoneId, locale: Locale): String? {
        val value = wire.getJSONObject("periodStatus")
        val prefix = value.getString("prefix").takeIf { it.isNotEmpty() } ?: return null
        if (value.isNull("timestamp")) return prefix
        return format(value.getString("timestamp"), zone, locale, value.getBoolean("includeTime"))
            ?.let { prefix + it }
    }

    /** The one quiet line under the limits: «Обновится в 03:00». */
    fun resetLine(zone: ZoneId, locale: Locale): String? = todayResetTime(zone, locale)?.let { "Обновится в $it" }

    /** «03:00»: when today's limit renews. */
    fun todayResetTime(zone: ZoneId, locale: Locale): String? =
        instant("todayReset")?.atZone(zone)?.let { DateTimeFormatter.ofPattern("HH:mm", locale).format(it) }

    /** «Новый период — с 29 ноября». */
    fun periodResetLine(zone: ZoneId, locale: Locale): String? =
        wire.optJSONObject("periodStatus")?.takeUnless { it.isNull("timestamp") }
            ?.let { runCatching { Instant.parse(it.getString("timestamp")) }.getOrNull() }
            ?.atZone(zone)?.let { "Новый период — с " + DateTimeFormatter.ofPattern("d MMMM", locale).format(it) }

    private fun instant(name: String): Instant? = if (wire.isNull(name)) null else runCatching { Instant.parse(wire.getString(name)) }.getOrNull()

    private fun percent(name: String): Int? = if (wire.isNull(name)) null
        else wire.getInt(name).also { require(it in 0..100) }

    private fun date(name: String, zone: ZoneId, locale: Locale, time: Boolean = false): String? =
        if (wire.isNull(name)) null else format(wire.getString(name), zone, locale, time)

    private fun format(value: String, zone: ZoneId, locale: Locale, time: Boolean): String? = runCatching {
        DateTimeFormatter.ofPattern(if (time) "d MMMM, HH:mm" else "d MMMM yyyy", locale)
            .format(Instant.parse(value).atZone(zone))
    }.getOrNull()
}

internal fun StyleAccess.quotaWire(): JSONObject = JSONObject()
    .put("type", type).put("state", state).put("planCode", planCode).put("balanceKind", balanceKind)
    .put("startsAt", startsAt).put("endsAt", endsAt).put("dailyResetsAt", dailyResetsAt)
    .put("nextPeriodAt", nextPeriodAt).put("periodResetCondition", periodResetCondition)
    .put("accessEndsAt", accessEndsAt)
