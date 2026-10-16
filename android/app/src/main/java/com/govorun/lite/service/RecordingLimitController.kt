package com.govorun.lite.service

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.widget.Toast
import com.govorun.lite.R
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.Haptics
import com.govorun.lite.shared.SharedDictationRules
import org.json.JSONObject

/** Applies one explicit whole-session limit without touching transcript data. */
internal class RecordingLimitController(
    private val context: Context,
    private val stopRecording: (silent: Boolean) -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private val stop = Runnable {
        Toast.makeText(context, R.string.stylization_recording_limit_reached, Toast.LENGTH_LONG)
            .show()
        stopRecording(false)
    }
    private val warning = Runnable {
        Toast.makeText(context, R.string.stylization_recording_limit_warning, Toast.LENGTH_LONG)
            .show()
        Haptics.doubleTap(context)
    }

    fun cancel() {
        handler.removeCallbacks(stop)
        handler.removeCallbacks(warning)
    }

    fun arm(officialReady: Boolean, streaming: Boolean, selfHosted: Boolean,
        keepScreenOn: Boolean, fallbackMinutes: Int) {
        cancel()
        val plan = try {
            SharedDictationRules.evaluate(JSONObject().put("kind", "recording").put("facts", JSONObject()
                .put("selfHosted", selfHosted).put("officialSelected", officialReady)
                .put("officialReady", officialReady).put("streaming", streaming)
                .put("localTimerEnabled", keepScreenOn).put("configuredMinutes", fallbackMinutes)))
        } catch (_: Exception) {
            // If the local bridge is unavailable, retain the explicit OS timer preference.
            if (keepScreenOn && fallbackMinutes > 0) handler.postDelayed(stop, fallbackMinutes.toLong() * 60_000)
            AppLog.log(context, "Service: recording timer bridge unavailable")
            return
        }
        if (!plan.isNull("warnAfterMillis")) handler.postDelayed(warning, plan.getLong("warnAfterMillis"))
        val duration = plan.getLong("stopAfterMillis")
        if (duration > 0) handler.postDelayed(stop, duration)
        AppLog.log(context, "Service: shared recording cap milliseconds=$duration")
    }
}
