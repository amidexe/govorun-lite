package com.govorun.lite.stats

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository

/** Application composition: notify only after the shared journal has been persisted. */
internal class AndroidStatsRecorder private constructor(context: Context) {
    private val app = context.applicationContext
    fun addWords(count: Int) = changed(StatsStore.addWords(app, count))
    fun addSeconds(seconds: Long) = changed(StatsStore.addSeconds(app, seconds))
    fun recordRecognitionPerformance(speechMillis: Long, processingMillis: Long, success: Boolean) =
        changed(StatsStore.recordRecognitionPerformance(app, speechMillis, processingMillis, success))
    private fun changed(date: String) { GovorunAccountRepository.recordStatsDayChanged(app, date) }
    companion object { fun create(context: Context) = AndroidStatsRecorder(context) }
}
