package com.govorun.lite.api

import com.govorun.lite.transcriber.RecognitionSessionArbiter

internal interface RecognitionProviderSession {
    val uid: Int
    val packageName: String
    val sessionId: String
    var recognitionLease: RecognitionSessionArbiter.Lease?

    fun cancel(notify: Boolean)
    fun cancelWithError(code: Int, message: String)

    fun attach(lease: RecognitionSessionArbiter.Lease) {
        recognitionLease = lease
    }

    fun preempt() {
        cancelWithError(
            RecognitionApi.ERROR_PREEMPTED,
            "Говорун начал собственную запись",
        )
    }
}
