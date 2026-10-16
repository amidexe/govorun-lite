package com.govorun.lite.account

import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

data class ActiveStyleLimitPresentation(
    val accessType: String,
    val planCode: String,
    val heading: String,
    val periodRange: String?,
    val todayLabel: String,
    val todayPercent: Int,
    val todayReset: String?,
    val periodLabel: String,
    val periodPercent: Int,
    val periodStatus: String?,
    val refreshing: Boolean = false,
) {
    val today: String get() = if (refreshing) "Обновляем остаток" else "$todayLabel — осталось $todayPercent%"
    val period: String get() = if (refreshing) "Обновляем остаток" else "$periodLabel — осталось $periodPercent%"
}

data class StarterStyleLimitPresentation(val percent: Int?) {
    val status: String get() = percent?.let { "Осталось $it%" } ?: "Обновляем остаток"
}

data class BonusStyleLimitPresentation(val percent: Int?, val expires: String?) {
    val balance: String get() = percent?.let { "Осталось $it%" } ?: "Обновляем остаток"
}

internal fun starterStyleLimitPresentation(state: AccountState): StarterStyleLimitPresentation? {
    val quota = state.styleWindowPresentation().quota ?: return null
    return if (quota.kind == "singleGrant") StarterStyleLimitPresentation(quota.periodPercent) else null
}

internal fun bonusStyleLimitPresentation(
    state: AccountState,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): BonusStyleLimitPresentation? {
    val quota = state.styleWindowPresentation().quota ?: return null
    return if (quota.kind == "singleBonus") BonusStyleLimitPresentation(
        quota.periodPercent, quota.periodStatus(zoneId, locale)) else null
}

internal fun activeStyleLimitPresentation(
    state: AccountState,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): ActiveStyleLimitPresentation? {
    val quota = state.styleWindowPresentation().quota ?: return null
    if (quota.kind != "period") return null
    return ActiveStyleLimitPresentation(
        accessType = state.styleAccess.type, planCode = state.styleAccess.planCode,
        heading = quota.heading, periodRange = quota.periodRange(zoneId, locale),
        todayLabel = "Сегодня", todayPercent = quota.todayPercent ?: 0,
        todayReset = quota.todayReset(zoneId, locale), periodLabel = "Общий лимит",
        periodPercent = quota.periodPercent ?: 0, periodStatus = quota.periodStatus(zoneId, locale),
        refreshing = quota.refreshing)
}

internal fun bonusAccessStatus(
    access: StyleAccess,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): String = formatTimestamp(access.endsAt, zoneId, locale, includeTime = false)
    ?.let { "Действует до $it" }
    ?: "Можно пользоваться сейчас"

internal fun bonusAccessTitle(state: AccountState): String =
    state.styleWindowPresentation().quota?.planTitle ?: "Доступ"

private fun formatTimestamp(
    value: String,
    zoneId: ZoneId,
    locale: Locale,
    includeTime: Boolean,
): String? = runCatching {
    val pattern = if (includeTime) "d MMMM, HH:mm" else "d MMMM yyyy"
    DateTimeFormatter.ofPattern(pattern, locale).format(Instant.parse(value).atZone(zoneId))
}.getOrNull()

internal fun AccountState.visibleStyleRemaining(daily: Boolean): Int {
    val projection = styleWindowPresentation()
    return if (daily) projection.remainingToday else projection.remainingTotal
}
