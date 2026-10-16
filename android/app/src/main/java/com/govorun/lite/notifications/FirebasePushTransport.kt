package com.govorun.lite.notifications

import android.content.Context
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailabilityLight
import com.google.android.gms.tasks.Task
import com.google.firebase.FirebaseApp
import com.google.firebase.FirebaseOptions
import com.google.firebase.installations.FirebaseInstallations
import com.google.firebase.messaging.FirebaseMessaging
import com.govorun.lite.BuildConfig
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/** The only provider-specific boundary; a RuStore adapter can implement it. */
internal interface PushTransport {
    val provider: String
    fun available(): Boolean
    suspend fun token(): String
    suspend fun remove()
}

internal class FirebasePushTransport(private val context: Context) : PushTransport {
    override val provider = "fcm"
    override fun available() = GoogleApiAvailabilityLight.getInstance()
        .isGooglePlayServicesAvailable(context) == ConnectionResult.SUCCESS

    private fun app(): FirebaseApp = FirebaseApp.getApps(context).firstOrNull { it.name == FirebaseApp.DEFAULT_APP_NAME }
        ?: FirebaseApp.initializeApp(context, FirebaseOptions.Builder()
            .setApplicationId(BuildConfig.FIREBASE_APP_ID).setApiKey(BuildConfig.FIREBASE_API_KEY)
            .setProjectId(BuildConfig.FIREBASE_PROJECT_ID).setGcmSenderId(BuildConfig.FIREBASE_SENDER_ID).build())
            .also { it.setDataCollectionDefaultEnabled(false) }

    override suspend fun token(): String {
        check(available())
        app()
        val messaging = FirebaseMessaging.getInstance()
        messaging.setDeliveryMetricsExportToBigQuery(false)
        // Token acquisition is always explicit. Persisted SDK auto-init must
        // never race a later consent revocation or a cleanup after restart.
        messaging.isAutoInitEnabled = false
        return messaging.token.awaitPrivate()
    }

    override suspend fun remove() {
        app()
        val messaging = FirebaseMessaging.getInstance()
        messaging.isAutoInitEnabled = false
        messaging.deleteToken().awaitPrivate()
        FirebaseInstallations.getInstance().delete().awaitPrivate()
    }
}

private suspend fun <T> Task<T>.awaitPrivate(): T = suspendCancellableCoroutine { continuation ->
    addOnCompleteListener { task ->
        if (continuation.isActive) {
            if (task.isSuccessful) continuation.resume(task.result)
            else continuation.resumeWithException(IllegalStateException("Notification provider unavailable"))
        }
    }
}
