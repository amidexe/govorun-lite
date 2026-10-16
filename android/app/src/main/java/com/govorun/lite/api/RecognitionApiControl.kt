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
    }
}
