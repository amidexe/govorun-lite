package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.InputMethod
import android.os.Build
import android.view.inputmethod.EditorInfo
import androidx.annotation.RequiresApi
import com.govorun.lite.BuildConfig
import com.govorun.lite.util.AppLog

/**
 * API-neutral boundary around AccessibilityService's input-method support.
 *
 * Android only added [InputMethod] in API 33. The service and TextInserter
 * must therefore never mention that type directly: ART loads those classes
 * on Android 12 and would fail while resolving the missing framework type.
 * The API-33 implementation below is instantiated only after an SDK check,
 * following the same isolated ApiXXImpl pattern used by AndroidX.
 */
internal interface InputMethodBridge {
    fun tryCommit(text: String): Boolean
    fun currentEditorInfo(): EditorInfo?
}

internal object InputMethodBridgeFactory {
    fun create(service: AccessibilityService): InputMethodBridge {
        if (BuildConfig.FORCE_ACCESSIBILITY_FALLBACK) {
            AppLog.log(service, "Paste: forced Android 12 fallback")
            return NoInputMethodBridge
        }
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Api33InputMethodBridge(service)
        } else {
            NoInputMethodBridge
        }
    }
}

private object NoInputMethodBridge : InputMethodBridge {
    override fun tryCommit(text: String): Boolean = false
    override fun currentEditorInfo(): EditorInfo? = null
}

@RequiresApi(Build.VERSION_CODES.TIRAMISU)
internal class Api33InputMethodBridge(
    private val service: AccessibilityService
) : InputMethodBridge {

    private var lastEditorInfo: EditorInfo? = null

    override fun tryCommit(text: String): Boolean {
        val connection = inputMethod()?.currentInputConnection ?: return false
        val surrounding = try {
            connection.getSurroundingText(1, 0, 0)?.text?.toString()
        } catch (_: Exception) {
            null
        }
        val toCommit = TextInsertionPlan.prependSpaceIfNeeded(text, surrounding)

        return try {
            connection.commitText(toCommit, 1, null)
            val verify = connection
                .getSurroundingText(toCommit.length, 0, 0)
                ?.text
                ?.toString()
            val landed = verify?.endsWith(toCommit) == true
            AppLog.log(
                service,
                if (landed) "Paste: commitText len=${toCommit.length}"
                else "Paste: commitText silently failed"
            )
            landed
        } catch (e: Exception) {
            AppLog.log(service, "Paste: commitText failed (${e.javaClass.simpleName})")
            false
        }
    }

    override fun currentEditorInfo(): EditorInfo? {
        val current = try { inputMethod()?.currentInputEditorInfo } catch (_: Exception) { null }
        if (current != null) lastEditorInfo = current
        return current ?: lastEditorInfo
    }

    private fun inputMethod(): InputMethod? = try {
        service.inputMethod
    } catch (_: Exception) {
        null
    }
}
