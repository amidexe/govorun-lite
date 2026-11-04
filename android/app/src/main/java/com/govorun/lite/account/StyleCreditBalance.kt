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
    /** Exact counts beside the percents: what is left of how many, today and in the period, for dictation. */
    val todayLeft: Int? = null,
    val todayLimit: Int? = null,
    val periodLeft: Int? = null,
    val periodLimit: Int? = null,
    /** The lines of Govorun's other products (Говорун Эхо, later others), each with its own limit, as the card has them. */
    val products: List<ProductLimitPresentation> = emptyList(),
    /** When the day and the period renew, in one quiet line. */
    val resetLine: String? = null,
    /** «Новый период — с 29 ноября». */
    val periodResetLine: String? = null,
) {
    val today: String get() = if (refreshing) "Обновляем остаток" else "$todayLabel — осталось $todayPercent%"
    val period: String get() = if (refreshing) "Обновляем остаток" else "$periodLabel — осталось $periodPercent%"
}

/** One product's line on the limits card: what is left of how many today and in the period, and the percents. */
data class ProductLimitPresentation(
    val product: String,
    val todayPercent: Int,
    val periodPercent: Int?,
    val todayLeft: Int,
    val todayLimit: Int,
    val periodLeft: Int,
    val periodLimit: Int?,
) {
    val title: String get() = com.govorun.lite.stylization.ProductLabel.title(product)
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

/** A product's own window: this phone's requests counted at once, else the card's line. */
internal fun AccountState.productWindow(product: String, window: StyleWindowPresentation = styleWindowPresentation()): ProductWindow? =
    window.products.firstOrNull { it.product == product } ?: styleAccess.products.firstOrNull { it.product == product && it.dailyLimit > 0 }
        ?.let { ProductWindow(it.product, it.dailyLimit, it.remainingToday, it.totalLimit, it.remainingTotal) }

internal fun activeStyleLimitPresentation(
    state: AccountState,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): ActiveStyleLimitPresentation? {
    val window = state.styleWindowPresentation()
    val quota = window.quota ?: return null
    if (quota.kind != "period") return null
    val products = (window.products.map { it.product } + state.styleAccess.products.map { it.product }).distinct()
        .mapNotNull { state.productWindow(it, window) }
    return ActiveStyleLimitPresentation(
        accessType = state.styleAccess.type, planCode = state.styleAccess.planCode,
        heading = quota.heading, periodRange = quota.periodRange(zoneId, locale),
        todayLabel = "Сегодня", todayPercent = quota.todayPercent ?: 0,
        todayReset = quota.todayReset(zoneId, locale), periodLabel = "Общий лимит",
        periodPercent = quota.periodPercent ?: 0, periodStatus = quota.periodStatus(zoneId, locale),
        refreshing = quota.refreshing,
        todayLeft = window.remainingToday, todayLimit = state.styleAccess.dailyLimit.takeIf { it > 0 },
        periodLeft = window.remainingTotal, periodLimit = state.styleAccess.totalLimit.takeIf { it > 0 },
        products = products.map { line -> ProductLimitPresentation(line.product, line.percentLeft, line.periodPercentLeft,
            line.remainingToday, line.limitToday, line.remainingTotal, line.limitTotal.takeIf { it > 0 }) },
        resetLine = quota.resetLine(zoneId, locale), periodResetLine = quota.periodResetLine(zoneId, locale))
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
