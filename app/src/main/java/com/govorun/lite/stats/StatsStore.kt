package com.govorun.lite.stats

import android.content.Context
import java.time.LocalDate

/**
 * Usage counters stored in SharedPreferences. Cheap single-process increments —
 * all writes happen from the accessibility service, reads from MainActivity.
 *
 * Also tracks a "сегодня" counter that rolls over on the local date change.
 * Implementation is lazy: we don't need a midnight alarm, we just snap to
 * today whenever addWords / getWordsToday is called and compare the stored
 * ISO date to the current one. If they don't match, today is treated as 0.
 */
object StatsStore {
    private const val PREFS = "govorun_lite_prefs"
    private const val KEY_WORDS = "stats_words_total"
    private const val KEY_SECONDS = "stats_seconds_total"
    private const val KEY_WORDS_TODAY = "stats_words_today"
    private const val KEY_TODAY_DATE = "stats_today_date"

    fun addWords(context: Context, count: Int) {
        if (count <= 0) return
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val current = prefs.getLong(KEY_WORDS, 0L)
        val today = LocalDate.now().toString()
        // If the stored date is a previous day, yesterday's count doesn't
        // carry into today — start today's tally from zero.
        val storedDate = prefs.getString(KEY_TODAY_DATE, null)
        val todayBefore = if (storedDate == today) prefs.getLong(KEY_WORDS_TODAY, 0L) else 0L
        prefs.edit()
            .putLong(KEY_WORDS, current + count)
            .putLong(KEY_WORDS_TODAY, todayBefore + count)
            .putString(KEY_TODAY_DATE, today)
            .apply()
    }

    fun addSeconds(context: Context, seconds: Long) {
        if (seconds <= 0L) return
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val current = prefs.getLong(KEY_SECONDS, 0L)
        prefs.edit().putLong(KEY_SECONDS, current + seconds).apply()
    }

    fun getWords(context: Context): Long =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getLong(KEY_WORDS, 0L)

    fun getSeconds(context: Context): Long =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getLong(KEY_SECONDS, 0L)

    fun getWordsToday(context: Context): Long {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val today = LocalDate.now().toString()
        val storedDate = prefs.getString(KEY_TODAY_DATE, null)
        return if (storedDate == today) prefs.getLong(KEY_WORDS_TODAY, 0L) else 0L
    }

    fun countWords(text: String): Int {
        if (text.isBlank()) return 0
        return text.trim().split(Regex("\\s+")).count { it.isNotEmpty() }
    }
}
