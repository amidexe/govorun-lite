package com.govorun.lite.stats

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository

/** Application composition: notify only after the shared journal has been persisted.
 * Statistics never stop a dictation: a count the base cannot keep is lost, the text is not.
 */
internal class AndroidStatsRecorder private constructor(context: Context) {
    private val app = context.applicationContext
    fun addWords(count: Int) = record { StatsStore.addWords(app, count) }
    fun addSeconds(seconds: Long) = record { StatsStore.addSeconds(app, seconds) }
    fun recordRecognitionPerformance(speechMillis: Long, processingMillis: Long, success: Boolean) =
        record { StatsStore.recordRecognitionPerformance(app, speechMillis, processingMillis, success) }
    private fun record(write: () -> String) {
        val date = try { write() } catch (_: RuntimeException) { return }
        GovorunAccountRepository.recordStatsDayChanged(app, date)
    }
    companion object { fun create(context: Context) = AndroidStatsRecorder(context) }
}
