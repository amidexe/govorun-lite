package com.govorun.lite

import android.app.Application
import com.google.android.material.color.DynamicColors
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.util.AppLog
import kotlin.concurrent.thread

class LiteApp : Application() {
    override fun onCreate() {
        super.onCreate()
        DynamicColors.applyToActivitiesIfAvailable(this)

        // Extract the bundled GigaAM model from APK assets to filesDir on a
        // background thread. sherpa-onnx needs filesystem paths; assets are
        // read-only. Runs in parallel with welcome/permission onboarding
        // steps so by the time the user reaches the try-it step the copy is
        // already done on all but the slowest devices.
        thread(name = "model-extract", isDaemon = true, priority = Thread.NORM_PRIORITY - 1) {
            try {
                GigaAmModel.ensureInstalled(this)
            } catch (t: Throwable) {
                AppLog.log(this, "Model extract failed: ${t.message}")
            }
        }
    }
}
