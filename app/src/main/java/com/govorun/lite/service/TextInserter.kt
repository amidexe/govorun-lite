package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.InputMethod
import android.os.Bundle
import android.view.accessibility.AccessibilityNodeInfo
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.transcriber.Dictionary
import com.govorun.lite.util.AppLog

/**
 * Inserts recognised text into whatever editable is currently focused.
 *
 * Two paths internally:
 *  1) Normal — IME pipeline via the accessibility-IME [InputConnection].
 *     This is what works in 99% of apps and runs without any side effects
 *     (no clipboard touches, no system toasts).
 *  2) Fallback — when the IME binding is "zombie" (some apps recycle their
 *     EditText without notifying us, leaving the binding pointing at a
 *     destroyed view). Detected by [InputConnection.getSurroundingText]
 *     returning null on a non-null connection. In that case we walk the
 *     accessibility tree fresh, find the focused editable, and write
 *     directly via ACTION_SET_TEXT — no clipboard either, so no
 *     "App pasted from clipboard" toast.
 *
 * Extracted from LiteAccessibilityService to keep that class under the
 * 500-line target. The service still owns the [LiteAccessibilityInputMethod]
 * lifecycle; this class just consumes it via [imeProvider] at insertion
 * time so we always pick up the latest reference.
 */
class TextInserter(
    private val service: AccessibilityService,
    private val imeProvider: () -> LiteAccessibilityInputMethod?
) {

    /**
     * Apply the user's autoreplace dictionary to [rawText], then insert
     * the result into the currently-focused editable. Updates the words
     * counter on success. No-op for blank text.
     */
    fun insert(rawText: String) {
        if (rawText.isBlank()) return
        // Dictionary pass before any commit attempt — fixes jargon /
        // proper-noun spellings the recogniser doesn't know. Empty
        // dictionary is a cheap no-op for users who haven't set one up.
        val replaced = Dictionary.applyReplacements(service, rawText)
        if (replaced != rawText) {
            AppLog.log(service, "Dictionary applied: '${rawText.take(40)}' → '${replaced.take(40)}'")
        }
        val connection = imeProvider()?.currentInputConnection
        if (connection != null) {
            // Zombie-binding probe: when Telegram (and similar apps) recycle
            // their input field, Android still hands us a fresh-looking
            // InputConnection but it's actually bound to the destroyed
            // EditText. commitText returns "success" silently and the text
            // never reaches the visible field. getSurroundingText returns
            // null on such a binding — that's the cheapest reliable sync
            // probe we have. Confirmed against repro on Telegram.
            val surrounding = try {
                connection.getSurroundingText(1, 0, 0)
            } catch (_: Exception) { null }
            if (surrounding != null) {
                val spacedText = prependSpaceIfNeeded(replaced, connection)
                connection.commitText(spacedText, 1, null)
                AppLog.log(service, "Paste: commitText len=${spacedText.length}")
                StatsStore.addWords(service, StatsStore.countWords(replaced))
                return
            }
        }
        // Fallback: bypass the IME pipeline entirely and write text via
        // ACTION_SET_TEXT on the focused editable node. This works even
        // when the IME binding is dead, because we resolve the target
        // node fresh from the accessibility tree on each call.
        if (setTextOnFocusedEditable(replaced)) {
            StatsStore.addWords(service, StatsStore.countWords(replaced))
        } else {
            AppLog.log(service, "Paste: SET_TEXT fallback failed — dropping '${replaced.take(40)}'")
        }
    }

    /**
     * Last-resort path when the IME binding is dead. Walks the active
     * accessibility tree, finds the focused editable, splices our text
     * into its current contents, and commits via ACTION_SET_TEXT +
     * ACTION_SET_SELECTION. Returns true on success.
     *
     * Hint-text handling matters here: a placeholder-only EditText can
     * return its placeholder string from node.text on some Android
     * versions and apps. Three independent signals are checked before
     * treating the field as logically empty (see inline comments).
     */
    private fun setTextOnFocusedEditable(text: String): Boolean {
        val root = service.rootInActiveWindow ?: return false
        val node = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            ?.takeIf { it.isEditable } ?: return false
        val hint = node.hintText?.toString() ?: ""
        val rawText = node.text?.toString() ?: ""
        // Several different ways an input can be "logically empty" but
        // still expose a non-empty text getter:
        //  1. isShowingHintText explicitly set (modern Android hint API)
        //  2. text == hintText (some apps set both to the placeholder)
        //  3. text non-empty but cursor at position 0 with no selection —
        //     this is the signature of Telegram-style placeholders, where
        //     the field returns its placeholder string from text but no
        //     real cursor has been placed yet (textSelectionStart/End
        //     stay at 0 instead of moving to the end of "real" content).
        // The third condition has a theoretical false-positive on a field
        // that genuinely has real text and the user happened to put their
        // cursor at position 0 — but that's uncommon and the cost
        // (text replaced instead of prepended) is much smaller than the
        // alternative of literally splicing "Сообщение" into messages.
        val placeholderByCursor = rawText.isNotEmpty() &&
            node.textSelectionStart <= 0 && node.textSelectionEnd <= 0
        val isPlaceholderShowing = node.isShowingHintText ||
            (hint.isNotEmpty() && rawText == hint) ||
            placeholderByCursor
        val current = if (isPlaceholderShowing) "" else rawText
        val rawCursor = node.textSelectionEnd
        val cursor = if (rawCursor in 0..current.length) rawCursor else current.length
        val before = current.substring(0, cursor)
        val after = current.substring(cursor)
        val needsLeadingSpace = before.isNotEmpty() && !before.last().isWhitespace()
        val toInsert = if (needsLeadingSpace) " $text" else text
        val newText = before + toInsert + after
        val setArgs = Bundle().apply {
            putCharSequence(
                AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                newText
            )
        }
        if (!node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, setArgs)) {
            return false
        }
        // Move the caret to the end of what we just inserted so the next
        // commit (or the user's typing) continues naturally instead of
        // landing back at the original cursor position.
        val newCursor = cursor + toInsert.length
        val selArgs = Bundle().apply {
            putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT, newCursor)
            putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT, newCursor)
        }
        node.performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION, selArgs)
        AppLog.log(service, "Paste: SET_TEXT len=${toInsert.length}")
        return true
    }

    private fun prependSpaceIfNeeded(
        text: String,
        connection: InputMethod.AccessibilityInputConnection
    ): String {
        val surrounding = connection.getSurroundingText(1, 0, 0)
        val lastChar = surrounding?.text?.toString()?.lastOrNull()
        return if (lastChar != null && !lastChar.isWhitespace()) " $text" else text
    }
}
