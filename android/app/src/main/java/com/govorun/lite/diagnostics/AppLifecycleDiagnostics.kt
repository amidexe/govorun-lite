package com.govorun.lite.diagnostics

import android.app.ActivityManager
import android.content.Context
import com.govorun.lite.stylization.LocalDiagnosticJournal
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlin.concurrent.thread

internal object AppLifecycleDiagnostics {
    private val connected = MutableStateFlow(false)
    val serviceConnected = connected.asStateFlow()

    fun collectPreviousExits(context: Context) {
        val app = context.applicationContext
        thread(name = "process-exit-diagnostics", isDaemon = true) {
            try {
                val manager = checkNotNull(app.getSystemService(ActivityManager::class.java))
                val records = manager.getHistoricalProcessExitReasons(app.packageName, 0, 32)
                    .filter { it.processName == app.packageName }
                    .map { ProcessExitRecord(it.timestamp, processExitReason(it.reason)) }
                LocalDiagnosticJournal.importProcessExits(app, records)
            } catch (_: Exception) {
                LocalDiagnosticJournal.record(app, LifecycleDiagnosticReason.EXIT_HISTORY_UNAVAILABLE)
            }
        }
    }

    fun onConnected(context: Context) {
        connected.value = true
        LocalDiagnosticJournal.record(context, LifecycleDiagnosticReason.SERVICE_CONNECTED)
    }

    fun onStopped(context: Context) {
        connected.value = false
        LocalDiagnosticJournal.record(context, LifecycleDiagnosticReason.SERVICE_STOPPED)
    }

    fun onInterrupted(context: Context) {
        LocalDiagnosticJournal.record(context, LifecycleDiagnosticReason.SERVICE_INTERRUPTED)
    }
}
