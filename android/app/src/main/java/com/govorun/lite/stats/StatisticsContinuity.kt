package com.govorun.lite.stats

import android.content.Context
import android.content.SharedPreferences
import java.security.MessageDigest
import org.json.JSONArray
import org.json.JSONObject

/** Read-only values from the preceding v3 store; all merge decisions stay in Rust. */
internal object StatisticsContinuity {
    fun read(context: Context, accountId: String?, today: String): JSONObject = read(
        context.applicationContext.getSharedPreferences("govorun_lite_prefs", Context.MODE_PRIVATE),
        context.applicationContext.getSharedPreferences("govorun_stats_sync_v2", Context.MODE_PRIVATE),
        accountId, today)

    internal fun read(local: SharedPreferences, sync: SharedPreferences, accountId: String?, today: String): JSONObject {
        val dates = local.getStringSet("stats_history_dates", emptySet()).orEmpty()
        val firstVersionDate = local.getString("stats_today_date", null)
            ?.takeIf { runCatching { java.time.LocalDate.parse(it) }.isSuccess }
        val buckets = listOf("lt025", "025to05", "05to1", "1to2", "gte2")
        val days = JSONArray(dates.sorted().map { date -> JSONObject()
            .put("date", date).put("revision", 1)
            .put("words", local.getLong("stats_words_day_$date", 0))
            .put("speechSeconds", local.getLong("stats_seconds_day_$date", 0))
            .put("recognitions", local.getLong("stats_recognitions_day_$date", 0))
            .put("recognitionErrors", local.getLong("stats_recognition_errors_day_$date", 0))
            .put("recognitionRtfBuckets", JSONArray(buckets.map { local.getLong("stats_recognition_rtf_${it}_day_$date", 0) })) })
        if (firstVersionDate != null && firstVersionDate !in dates) {
            days.put(JSONObject().put("date", firstVersionDate).put("revision", 1)
                .put("words", local.getLong("stats_words_today", 0))
                .put("speechSeconds", 0).put("recognitions", 0)
                .put("recognitionErrors", 0).put("recognitionRtfBuckets", JSONArray(List(5) { 0 })))
        }
        val total = JSONObject().put("words", local.getLong("stats_words_total", 0))
            .put("speechSeconds", local.getLong("stats_seconds_total", 0))
            .put("recognitions", local.getLong("stats_recognitions_total", 0))
        val account = accountId?.let { id ->
            val hash = MessageDigest.getInstance("SHA-256").digest(id.toByteArray(Charsets.UTF_8))
                .take(12)
                .joinToString("") { "%02x".format(it) }
            val acknowledged = JSONArray(sync.getString("acknowledged_$hash", "[]"))
            for (index in 0 until acknowledged.length()) acknowledged.getJSONObject(index).put("revision", 1)
            JSONObject().put("id", id).put("generation", sync.getLong("generation_$hash", 0))
                .put("acknowledgedTotal", JSONObject(sync.getString("acknowledged_total_$hash",
                    "{\"words\":0,\"speechSeconds\":0,\"recognitions\":0}")))
                .put("acknowledgedDays", acknowledged).put("resetPending", sync.getBoolean("reset_$hash", false))
        }
        return JSONObject().put("today", today).put("total", total).put("days", days)
            .put("account", account ?: JSONObject.NULL)
    }
}
