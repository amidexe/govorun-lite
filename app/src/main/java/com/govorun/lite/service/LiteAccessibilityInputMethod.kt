package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.InputMethod
import android.util.Log
import android.view.inputmethod.EditorInfo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeoutOrNull

/**
 * AccessibilityService InputMethod that tracks how many times Android has
 * fired onStartInput / onFinishInput for our binding. Used by [pasteText]
 * to detect "zombie" InputConnection state — when Android hands us a
 * fresh-looking InputConnection object that's actually bound to a destroyed
 * EditText, commitText silently succeeds but the text never appears.
 *
 * The pattern is: count startInput callbacks; before commit, await the
 * next callback with a small timeout. If we observe a fresh callback
 * within the window, the binding is alive. If we time out, the binding
 * is stale and the commit would be lost — caller falls back to the
 * accessibility-tree path instead.
 *
 * We also wrap onStartInput in try/catch for NullPointerException, which
 * the framework can throw from its own bookkeeping during binding
 * restarts on certain Android versions. The crash guard at process level
 * would catch it anyway, but absorbing it locally lets the rest of
 * the input session proceed instead of being torn down.
 *
 * Pattern adopted from Wispr Flow's WisprInputMethod (decompiled from
 * v1.8.8). Their app has been shipping this approach for two years
 * across half a million installs to handle WhatsApp/Telegram input
 * field recycling — the same root cause as our open Telegram zombie
 * binding bug.
 */
class LiteAccessibilityInputMethod(
    service: AccessibilityService
) : InputMethod(service) {

    companion object {
        private const val TAG = "LiteIME"
    }

    // Monotonically incremented every time Android delivers an onStartInput
    // callback. Both writes happen on the framework's dispatch thread (main).
    // Reads happen from the paste path (also main). MutableStateFlow gives
    // us coroutine-friendly suspending await for "value > N".
    private val startCount = MutableStateFlow(0L)
    private val finishCount = MutableStateFlow(0L)

    /** True if Android currently considers the input session active
     *  (more starts than finishes). Approximate — this is a snapshot. */
    val isInputSessionActive: Boolean
        get() = startCount.value > finishCount.value

    /** Number of onStartInput callbacks we've received since this
     *  InputMethod instance was created. Used by paste path to detect
     *  fresh binding without waiting if a start has already occurred
     *  since last paste. */
    val currentStartCount: Long
        get() = startCount.value

    /**
     * Last EditorInfo seen in onStartInput. Intentionally NOT cleared in
     * onFinishInput — Android nulls out currentInputEditorInfo the moment
     * the input session ends, but the IME window stays visible for another
     * ~200-400 ms during its closing animation. Without this fallback,
     * InputFieldFilter sees (imeVisible=true, searchField=false) and briefly
     * shows the bubble before the window fully disappears. Keeping
     * lastEditorInfo lets the filter still report "this was a search field"
     * through the closing gap, and it is replaced the next time onStartInput
     * fires for a new field.
     */
    var lastEditorInfo: EditorInfo? = null
        private set

    override fun onStartInput(attribute: EditorInfo, restarting: Boolean) {
        lastEditorInfo = attribute
        try {
            super.onStartInput(attribute, restarting)
        } catch (npe: NullPointerException) {
            // Framework NPE during its own binding bookkeeping. The global
            // crash guard would also catch this, but absorbing it here lets
            // us still bump the start counter and keep the session usable.
            Log.w(TAG, "onStartInput suppressed framework NPE: ${npe.message}")
        }
        startCount.value = startCount.value + 1
    }

    override fun onFinishInput() {
        try {
            super.onFinishInput()
        } catch (npe: NullPointerException) {
            Log.w(TAG, "onFinishInput suppressed framework NPE: ${npe.message}")
        }
        finishCount.value = finishCount.value + 1
    }

    /**
     * Suspends until the start counter exceeds [sinceCount], or until
     * [timeoutMs] elapses. Returns true if a fresh onStartInput was
     * observed (binding is alive), false on timeout (binding is most
     * likely a zombie reference to a destroyed EditText).
     *
     * Caller pattern:
     * ```
     * val before = ime.currentStartCount
     * connection.commitText(...)  // or skip and fallback
     * val ok = ime.awaitNextInputStart(before, 500L)
     * ```
     * Or, to gate the commit itself on a fresh binding, snapshot before
     * the user starts speaking and await between recognition and commit.
     */
    suspend fun awaitNextInputStart(sinceCount: Long, timeoutMs: Long): Boolean {
        if (startCount.value > sinceCount) return true
        val result = withTimeoutOrNull(timeoutMs) {
            startCount.first { it > sinceCount }
        }
        return result != null
    }
}
