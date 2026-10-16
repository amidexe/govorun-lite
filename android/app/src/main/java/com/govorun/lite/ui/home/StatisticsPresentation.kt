package com.govorun.lite.ui.home

import kotlin.math.abs
import kotlin.math.roundToLong

internal data class StatisticsSummary(
    val todayWords: Long,
    val weekWords: Long,
    val totalWords: Long,
    val weekSpeechSeconds: Long,
    val totalSpeechSeconds: Long,
    val activeDays: Int,
    val peakWords: Long,
    val previousWeekWords: Long = 0,
) {
    val comparison: String get() = weekComparison(weekWords, previousWeekWords)
    val hasWeeklyActivity: Boolean get() = activeDays > 0
}

internal fun StatisticsState.toSummary(): StatisticsSummary {
    val recent = wordHistory.takeLast(14)
    val currentWeek = recent.takeLast(7)
    val currentWords = currentWeek.sum()
    return StatisticsSummary(
        todayWords = wordsToday,
        weekWords = currentWords,
        totalWords = wordsTotal,
        weekSpeechSeconds = voiceSecondHistory.takeLast(7).sum(),
        totalSpeechSeconds = voiceSeconds,
        activeDays = currentWeek.count { it > 0L },
        peakWords = currentWeek.maxOrNull() ?: 0L,
        previousWeekWords = recent.dropLast(7).sum(),
    )
}

internal fun Long.wordsLabel(): String {
    if (this >= 1_000L) return "${compactNumber()} слов"
    val form = when {
        this % 10 == 1L && this % 100 != 11L -> "слово"
        this % 10 in 2L..4L && this % 100 !in 12L..14L -> "слова"
        else -> "слов"
    }
    return "$this $form"
}

internal fun Long.speechDurationLabel(): String = when {
    this <= 0L -> "0 сек"
    this < 60L -> "$this сек"
    this < 3_600L -> "${this / 60L} мин"
    this % 3_600L < 60L -> "${this / 3_600L} ч"
    else -> "${this / 3_600L} ч ${(this % 3_600L) / 60L} мин"
}

internal fun weekComparison(current: Long, previous: Long): String {
    if (previous == 0L) return if (current == 0L) "Пока нет данных для сравнения"
        else "В предыдущие 7 дней — 0 слов"
    if (current == previous) return "Столько же, сколько за предыдущие 7 дней"
    val percent = (abs(current.toDouble() / previous - 1) * 100).roundToLong().coerceAtLeast(1)
    val direction = if (current > previous) "больше" else "меньше"
    return "На $percent% $direction, чем за предыдущие 7 дней"
}
