package com.govorun.lite.stats

import android.content.Context
import java.time.LocalDate
import org.json.JSONObject

/** Android storage/value binding. Counter, acknowledgement and merge rules live in Rust. */
object StatsStore {
    internal fun addWords(context: Context, count: Int): String = record(context,
        JSONObject().put("kind", "words").put("count", count))

    internal fun addSeconds(context: Context, seconds: Long): String = record(context,
        JSONObject().put("kind", "seconds").put("seconds", seconds))

    internal fun recordRecognitionPerformance(context: Context, speechMillis: Long,
        processingMillis: Long, success: Boolean): String = record(context,
        JSONObject().put("kind", "performance").put("speechMilliseconds", speechMillis)
            .put("processingMilliseconds", processingMillis).put("success", success))

    private fun record(context: Context, event: JSONObject): String {
        val date = LocalDate.now().toString()
        StatisticsJournal.step(context, JSONObject().put("kind", "record").put("today", date).put("event", event))
        return date
    }

    fun getWords(context: Context): Long = getLifetimeStats(context).words
    fun getSeconds(context: Context): Long = getLifetimeStats(context).speechSeconds
    fun getRecognitions(context: Context): Long = getLifetimeStats(context).recognitions
    fun getLifetimeStats(context: Context): LocalStatsTotal {
        val value = StatisticsJournal.snapshot(context).getJSONObject("state").getJSONObject("total")
        return LocalStatsTotal(value.getLong("words"), value.getLong("speechSeconds"), value.getLong("recognitions"))
    }
    fun getWordsToday(context: Context): Long = getAllDailyStats(context)
        .firstOrNull { it.date == LocalDate.now().toString() }?.words ?: 0

    fun getAllDailyStats(context: Context): List<LocalStatsDay> {
        val values = StatisticsJournal.snapshot(context).getJSONObject("state").getJSONObject("days")
        return values.keys().asSequence().sorted().map { key ->
            val day = values.getJSONObject(key)
            val buckets = day.getJSONArray("recognitionRtfBuckets")
            LocalStatsDay(key, day.getLong("words"), day.getLong("speechSeconds"),
                day.getLong("recognitions"), List(buckets.length()) { buckets.getLong(it) },
                day.getLong("recognitionErrors"))
        }.toList()
    }
    fun getRecentDailyStats(context: Context, days: Int = 35): List<LocalStatsDay> {
        val end = LocalDate.now()
        val start = end.minusDays((days.coerceIn(1, 35) - 1).toLong()).toString()
        return getAllDailyStats(context).filter { it.date >= start && it.date <= end.toString() }
    }
    fun getWordHistory(context: Context, days: Int = 7): List<Long> = history(context, days) { it.words }
    fun getSecondHistory(context: Context, days: Int = 7): List<Long> = history(context, days) { it.speechSeconds }
    private fun history(context: Context, days: Int, value: (LocalStatsDay) -> Long): List<Long> {
        val byDate = getAllDailyStats(context).associateBy { it.date }
        val today = LocalDate.now()
        return (days.coerceIn(1, 31) - 1 downTo 0).map { offset ->
            byDate[today.minusDays(offset.toLong()).toString()]?.let(value) ?: 0L
        }
    }
    fun countWords(text: String): Int = StatisticsJournal.evaluate(JSONObject.NULL,
        JSONObject().put("kind", "countWords").put("text", text)).getInt("count")

    fun reset(context: Context) {
        StatisticsJournal.step(context, JSONObject().put("kind", "reset").put("account", JSONObject.NULL))
    }
}

data class LocalStatsTotal(val words: Long, val speechSeconds: Long, val recognitions: Long)

data class LocalStatsDay(val date: String, val words: Long, val speechSeconds: Long,
    val recognitions: Long, val recognitionRtfBuckets: List<Long>, val recognitionErrors: Long)
