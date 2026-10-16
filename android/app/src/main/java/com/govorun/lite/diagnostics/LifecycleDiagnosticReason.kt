package com.govorun.lite.diagnostics

import android.app.ApplicationExitInfo

/** Only these fixed values may enter the local journal. No system descriptions or traces. */
internal enum class LifecycleDiagnosticReason {
    CHECK_COMPLETED, CHECK_SAMPLES_FAILED, CHECK_MODEL_FAILED, CHECK_RECOGNITION_FAILED,
    SERVICE_CONNECTED, SERVICE_STOPPED, SERVICE_INTERRUPTED,
    EXIT_LOW_MEMORY, EXIT_CRASH, EXIT_NATIVE_CRASH, EXIT_ANR, EXIT_SELF,
    EXIT_SIGNALED, EXIT_INITIALIZATION, EXIT_PERMISSION_CHANGED, EXIT_RESOURCE_USAGE,
    EXIT_USER_REQUESTED, EXIT_USER_STOPPED, EXIT_DEPENDENCY, EXIT_OTHER, EXIT_FREEZER,
    EXIT_PACKAGE_CHANGED, EXIT_PACKAGE_UPDATED, EXIT_UNKNOWN, EXIT_HISTORY_UNAVAILABLE,
}

internal fun processExitReason(reason: Int): LifecycleDiagnosticReason = when (reason) {
    ApplicationExitInfo.REASON_LOW_MEMORY -> LifecycleDiagnosticReason.EXIT_LOW_MEMORY
    ApplicationExitInfo.REASON_CRASH -> LifecycleDiagnosticReason.EXIT_CRASH
    ApplicationExitInfo.REASON_CRASH_NATIVE -> LifecycleDiagnosticReason.EXIT_NATIVE_CRASH
    ApplicationExitInfo.REASON_ANR -> LifecycleDiagnosticReason.EXIT_ANR
    ApplicationExitInfo.REASON_EXIT_SELF -> LifecycleDiagnosticReason.EXIT_SELF
    ApplicationExitInfo.REASON_SIGNALED -> LifecycleDiagnosticReason.EXIT_SIGNALED
    ApplicationExitInfo.REASON_INITIALIZATION_FAILURE -> LifecycleDiagnosticReason.EXIT_INITIALIZATION
    ApplicationExitInfo.REASON_PERMISSION_CHANGE -> LifecycleDiagnosticReason.EXIT_PERMISSION_CHANGED
    ApplicationExitInfo.REASON_EXCESSIVE_RESOURCE_USAGE -> LifecycleDiagnosticReason.EXIT_RESOURCE_USAGE
    ApplicationExitInfo.REASON_USER_REQUESTED -> LifecycleDiagnosticReason.EXIT_USER_REQUESTED
    ApplicationExitInfo.REASON_USER_STOPPED -> LifecycleDiagnosticReason.EXIT_USER_STOPPED
    ApplicationExitInfo.REASON_DEPENDENCY_DIED -> LifecycleDiagnosticReason.EXIT_DEPENDENCY
    ApplicationExitInfo.REASON_OTHER -> LifecycleDiagnosticReason.EXIT_OTHER
    ApplicationExitInfo.REASON_FREEZER -> LifecycleDiagnosticReason.EXIT_FREEZER
    ApplicationExitInfo.REASON_PACKAGE_STATE_CHANGE -> LifecycleDiagnosticReason.EXIT_PACKAGE_CHANGED
    ApplicationExitInfo.REASON_PACKAGE_UPDATED -> LifecycleDiagnosticReason.EXIT_PACKAGE_UPDATED
    else -> LifecycleDiagnosticReason.EXIT_UNKNOWN
}

internal data class ProcessExitRecord(val timeMillis: Long, val reason: LifecycleDiagnosticReason)

internal fun unseenProcessExits(records: List<ProcessExitRecord>, checkpoint: Long): List<ProcessExitRecord> =
    records.filter { it.timeMillis > maxOf(0, checkpoint) }
        .distinct().sortedBy { it.timeMillis }.takeLast(32)
