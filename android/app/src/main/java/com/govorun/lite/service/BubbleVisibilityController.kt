package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.app.KeyguardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityWindowInfo
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.Prefs

internal class BubbleVisibilityController(
    private val service: AccessibilityService,
    private val overlay: () -> BubbleOverlayController?,
    private val isRecording: () -> Boolean,
    private val recordingMode: () -> BubbleOverlayController.RecordingUiMode,
    private val stopRecording: () -> Unit,
    private val editorInfo: () -> EditorInfo?,
) {
    var isImeVisible: Boolean = false
        private set

    val currentForegroundPackage: String?
        get() = foregroundPackage

    @Volatile
    private var alwaysShow = false

    @Volatile
    private var silence = false

    private var foregroundPackage: String? = null
    private val keyguard = service.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
    private val showHandler = Handler(Looper.getMainLooper())
    private val pendingShow = Runnable { overlay()?.setVisibility(View.VISIBLE) }

    fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) {
            trackForegroundPackage(event)
        }
        if (
            event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED ||
            event.eventType == AccessibilityEvent.TYPE_WINDOWS_CHANGED ||
            event.eventType == AccessibilityEvent.TYPE_VIEW_FOCUSED
        ) {
            update()
        }
    }

    fun update() {
        val imeVisible = runCatching {
            service.windows?.any { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD } == true
        }.getOrDefault(false)
        val locked = keyguard.isKeyguardLocked
        val currentEditor = editorInfo()
        val passwordField = InputFieldFilter.isPasswordField(currentEditor)
        val searchField = Prefs.isHideInSearchEnabled(service) &&
            InputFieldFilter.isSearchField(currentEditor)
        findLiveForegroundPackage()?.let { foregroundPackage = it }
        val appFiltered = Prefs.isAppFiltered(service, foregroundPackage)
        val normalShouldShow =
            imeVisible && !locked && !passwordField && !searchField && !appFiltered
        val portableRecording = isRecording() &&
            recordingMode() != BubbleOverlayController.RecordingUiMode.LEGACY
        val shouldShow = normalShouldShow || (portableRecording && !locked && !passwordField)
        val visibilityChanged = shouldShow != isImeVisible
        if (!visibilityChanged && !alwaysShow) return
        if (visibilityChanged && !shouldShow) stopRecording()
        isImeVisible = shouldShow
        val effectiveVisibility = when {
            silence -> View.GONE
            alwaysShow && !locked && !passwordField -> View.VISIBLE
            shouldShow -> View.VISIBLE
            else -> View.GONE
        }
        applyWithDebounce(effectiveVisibility)
        AppLog.log(
            service,
            "Service: bubbleShow=$shouldShow ime=$imeVisible locked=$locked " +
                "password=$passwordField search=$searchField appFiltered=$appFiltered " +
                "alwaysShow=$alwaysShow silence=$silence foreground=$foregroundPackage",
        )
    }

    fun isAlwaysShowEnabled() = alwaysShow

    fun isSilenceEnabled() = silence

    fun toggleAlwaysShow() {
        alwaysShow = !alwaysShow
        applyNow()
    }

    fun toggleSilence() {
        silence = !silence
        applyNow()
    }

    fun destroy() {
        showHandler.removeCallbacks(pendingShow)
    }

    private fun trackForegroundPackage(event: AccessibilityEvent) {
        val packageName = event.packageName?.toString() ?: return
        if (packageName == service.packageName || isImePackage(packageName)) return
        val sourceType = runCatching { event.source?.window?.type }.getOrNull()
        if (sourceType != AccessibilityWindowInfo.TYPE_SYSTEM) foregroundPackage = packageName
    }

    private fun findLiveForegroundPackage(): String? = runCatching {
        service.windows
            ?.asSequence()
            ?.filter { it.type == AccessibilityWindowInfo.TYPE_APPLICATION }
            ?.mapNotNull { it.root?.packageName?.toString() }
            ?.firstOrNull { it != service.packageName && !isImePackage(it) }
    }.getOrNull()

    private fun isImePackage(packageName: String): Boolean {
        val manager = service.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            ?: return false
        return manager.enabledInputMethodList.any { it.packageName == packageName }
    }

    private fun applyWithDebounce(visibility: Int) {
        showHandler.removeCallbacks(pendingShow)
        if (visibility == View.VISIBLE) {
            showHandler.postDelayed(pendingShow, SHOW_DEBOUNCE_MILLIS)
        } else {
            overlay()?.setVisibility(visibility)
        }
    }

    private fun applyNow() {
        val locked = keyguard.isKeyguardLocked
        val passwordField = InputFieldFilter.isPasswordField(editorInfo())
        val visibility = when {
            silence -> View.GONE
            locked || passwordField -> View.GONE
            alwaysShow -> View.VISIBLE
            isImeVisible -> View.VISIBLE
            else -> View.GONE
        }
        showHandler.removeCallbacks(pendingShow)
        overlay()?.setVisibility(visibility)
    }

    private companion object {
        const val SHOW_DEBOUNCE_MILLIS = 120L
    }
}
