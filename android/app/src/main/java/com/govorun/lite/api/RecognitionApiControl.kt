package com.govorun.lite.api

import android.content.Context

/** Coordinates per-application revocation with the live provider. */
internal object RecognitionApiControl {
    @Volatile private var activeService: RecognitionService? = null

    fun attach(service: RecognitionService) {
        activeService = service
    }

    fun detach(service: RecognitionService) {
        if (activeService === service) activeService = null
    }

    fun revoke(context: Context, packageName: String) {
        RecognitionAuthorizationStore(context).revoke(packageName)
        activeService?.cancelPackage(packageName)
        com.govorun.lite.api.smart.SmartRuntime.tasks(context).cancelAll(packageName)
    }

    /** «Кнопка громкости» for one approved app. */
    fun setVolumeKey(context: Context, packageName: String, enabled: Boolean) {
        RecognitionAuthorizationStore(context).setScope(packageName, RecognitionApi.SCOPE_VOLUME_KEY, enabled)
    }

    /** Off: the app's own smart tasks stop going; the person's own press turns it on again. */
    fun setSmartFunctions(context: Context, packageName: String, enabled: Boolean) {
        RecognitionAuthorizationStore(context).setScope(packageName, RecognitionApi.SCOPE_SMART_FUNCTIONS, enabled)
        if (!enabled) com.govorun.lite.api.smart.SmartRuntime.tasks(context).cancelAutomatic(packageName)
    }
}
