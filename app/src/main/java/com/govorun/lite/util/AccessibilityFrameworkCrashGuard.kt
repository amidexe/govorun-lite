package com.govorun.lite.util

import android.util.Log

/**
 * Suppresses known Android framework crashes that occur on the
 * accessibility-IME pipeline thread.
 *
 * Without this, an internal NullPointerException in the system's own
 * InputMethod / AccessibilityInputMethodSession bookkeeping (during a
 * binding restart, for example) propagates up through our coroutines
 * to the default uncaught handler — which kills our entire process.
 * Android then refuses to auto-restart the accessibility service after
 * a few rapid faults, leaving the user with a "running" Settings entry
 * but a dead bubble until they manually re-toggle accessibility.
 *
 * The classes and methods listed below are the framework call sites
 * known to throw NPE on internal Android bugs we cannot fix from
 * userspace — and the StackOverflow path is the long-standing
 * AccessibilityCache recursion that hits cycle-shaped node graphs.
 *
 * Pattern adopted from Wispr Flow's AccessibilityFrameworkCrashGuard
 * (decompiled 2026-04-30 from APK v1.8.8). They've been shipping it
 * for two years across half a million installs, so the list of
 * "known" crashes here reflects what they had to add over time —
 * we inherit that list rather than rediscover it ourselves.
 */
object AccessibilityFrameworkCrashGuard {

    private const val TAG = "A11yCrashGuard"

    private val FRAMEWORK_CRASH_CLASSES = arrayOf(
        "android.accessibilityservice.InputMethod\$SessionImpl",
        "android.accessibilityservice.AccessibilityInputMethodSessionWrapper",
        "android.accessibilityservice.AccessibilityService\$IAccessibilityServiceClientWrapper",
        "android.view.accessibility.AccessibilityNodeInfo"
    )

    private val FRAMEWORK_CRASH_METHODS = arrayOf(
        "invalidateInput",
        "doInvalidateInput",
        "startInput",
        "writeToParcelNoRecycle"
    )

    private const val ACCESSIBILITY_CACHE_CLASS = "android.view.accessibility.AccessibilityCache"
    private const val ACCESSIBILITY_CACHE_METHOD = "isCachedNodeOrDescendantLocked"

    @Volatile private var previousHandler: Thread.UncaughtExceptionHandler? = null
    @Volatile private var installed = false

    /**
     * Install on the calling thread. Idempotent — second call is a no-op.
     * Call from AccessibilityService.onServiceConnected (main thread): the
     * accessibility-IME binder callbacks are dispatched onto that thread,
     * which is where the framework NPEs surface.
     */
    @Synchronized
    fun install() {
        if (installed) return
        previousHandler = Thread.currentThread().uncaughtExceptionHandler
        Thread.currentThread().uncaughtExceptionHandler = Handler()
        installed = true
    }

    @Synchronized
    fun uninstall() {
        if (!installed) return
        Thread.currentThread().uncaughtExceptionHandler = previousHandler
        previousHandler = null
        installed = false
    }

    private class Handler : Thread.UncaughtExceptionHandler {
        override fun uncaughtException(thread: Thread, throwable: Throwable) {
            if (!isKnownFrameworkCrash(throwable)) {
                // Not on our list — let the original handler do its thing
                // (which is normally killing the process; that's the right
                // behaviour for actual bugs in our own code, just not for
                // framework bookkeeping NPEs).
                previousHandler?.uncaughtException(thread, throwable)
                return
            }
            Log.w(
                TAG,
                "Suppressed known framework crash: " +
                    "${throwable.javaClass.simpleName}: ${throwable.message}"
            )
            // Deliberately swallow. The framework will recover its own
            // state on the next event; we just need to stay alive long
            // enough to receive that event.
        }
    }

    private fun isKnownFrameworkCrash(throwable: Throwable): Boolean {
        if (throwable is StackOverflowError) {
            return isAccessibilityCacheOverflow(throwable)
        }
        if (throwable !is NullPointerException) return false
        val stack = throwable.stackTrace
        // Only the top of the stack matters — if the framework call wasn't
        // very near the throw site, this isn't the bug we're protecting against.
        val limit = stack.size.coerceAtMost(5)
        for (i in 0 until limit) {
            val element = stack[i]
            val className = element.className ?: continue
            val methodName = element.methodName ?: continue
            for (knownClass in FRAMEWORK_CRASH_CLASSES) {
                val matchesClass = className == knownClass ||
                    className.startsWith("$knownClass$")
                if (!matchesClass) continue
                for (knownMethod in FRAMEWORK_CRASH_METHODS) {
                    if (methodName == knownMethod) return true
                }
            }
        }
        return false
    }

    private fun isAccessibilityCacheOverflow(error: StackOverflowError): Boolean {
        val stack = error.stackTrace
        if (stack.isEmpty()) return false
        // AccessibilityCache.isCachedNodeOrDescendantLocked recurses on
        // node parents; on cycle-shaped graphs (some custom WebViews,
        // certain Compose trees) it overflows. Surface signature: that
        // exact method appears repeatedly near the top of the stack.
        val limit = stack.size.coerceAtMost(20)
        for (i in 0 until limit) {
            val element = stack[i]
            if (element.className == ACCESSIBILITY_CACHE_CLASS &&
                element.methodName == ACCESSIBILITY_CACHE_METHOD
            ) {
                return true
            }
        }
        return false
    }
}
