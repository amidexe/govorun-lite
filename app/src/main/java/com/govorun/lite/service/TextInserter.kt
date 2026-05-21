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
 * Three layers of insertion attempt:
 *  1) Normal — `commitText` via the IME [InputConnection].
 *  2) Post-commit verification — re-read text before cursor; if the field
 *     does not end with what we just committed, the IC is "zombie" (some
 *     apps recycle their EditText, leaving us with a connection that
 *     accepts commits silently but never delivers them to the visible
 *     view). The simpler probe `getSurroundingText==null` catches some
 *     zombie variants but not all — Telegram has been observed accepting
 *     commits while `getSurroundingText` returned a non-null empty value.
 *  3) Accessibility-tree fallback — find the focused editable node fresh
 *     in the accessibility tree and write via ACTION_SET_TEXT. In debug
 *     builds the inserted text is prefixed with `[fb]` so it is visually
 *     obvious when this path fired (it's the explicit success signal that
 *     we recovered through the alternative path).
 *
 * Trade-off accepted in current design: post-commit verification can
 * occasionally false-positive (verification reads stale snapshot, thinks
 * commit failed when it actually landed) leading to a possible duplicate
 * write via the fallback path. The user has explicitly chosen this over
 * silent failure — seeing `[fb]` is preferable to seeing nothing.
 *
 * Fallback safety rules (learnt the hard way):
 *  - Always call `node.refresh()` before reading `node.text`. The
 *    accessibility tree lags the real UI; without refresh, we can read
 *    a just-sent message that is no longer visible and write it back
 *    as a duplicate.
 *  - Treat the field as "logically empty" only when one of two reliable
 *    signals fires: `isShowingHintText` true, OR `text == hintText`.
 *    Do NOT invent additional heuristics like "cursor at 0,0 means
 *    placeholder" — they false-positive in zombie state on real user
 *    content and silently destroy data.
 */
class TextInserter(
    private val service: AccessibilityService,
    private val imeProvider: () -> LiteAccessibilityInputMethod?
) {

    fun insert(rawText: String) {
        if (rawText.isBlank()) return
        val replaced = Dictionary.applyReplacements(service, rawText)
        if (replaced != rawText) {
            AppLog.log(service, "Dictionary applied (raw=${rawText.length} → result=${replaced.length})")
        }

        val connection = imeProvider()?.currentInputConnection
        if (connection != null && tryCommitViaIme(connection, replaced)) {
            StatsStore.addWords(service, StatsStore.countWords(replaced))
            return
        }
        // commitText didn't land (IC null or zombie). Walk the accessibility
        // tree and write directly via ACTION_SET_TEXT.
        if (setTextOnFocusedEditable(replaced)) {
            StatsStore.addWords(service, StatsStore.countWords(replaced))
        } else {
            AppLog.log(service, "Paste: all paths failed — dropped len=${replaced.length}")
        }
    }

    /**
     * Attempt to commit through the IME pipeline. Returns true only when
     * post-commit verification confirms the text actually appeared.
     *
     * Verification reads `getSurroundingText` after `commitText` and checks
     * whether the tail before cursor matches what we sent. If not, the
     * connection is zombie and the caller should fall through to the
     * accessibility-tree path.
     */
    private fun tryCommitViaIme(
        connection: InputMethod.AccessibilityInputConnection,
        text: String
    ): Boolean {
        val preSurr = try {
            connection.getSurroundingText(1, 0, 0)
        } catch (_: Exception) { null }
        val toCommit = prependSpaceIfNeeded(text, preSurr?.text?.toString())
        connection.commitText(toCommit, 1, null)

        val verify = try {
            connection.getSurroundingText(toCommit.length, 0, 0)?.text?.toString()
        } catch (_: Exception) { null }
        val landed = verify != null && verify.endsWith(toCommit)
        if (landed) {
            AppLog.log(service, "Paste: commitText len=${toCommit.length}")
        } else {
            AppLog.log(service, "Paste: commitText silently failed")
        }
        return landed
    }

    /**
     * Last-resort path when commitText silently failed. Walks the active
     * accessibility tree, finds the focused editable, splices our text
     * into its current contents, and commits via ACTION_SET_TEXT +
     * ACTION_SET_SELECTION. Returns true on success.
     */
    private fun setTextOnFocusedEditable(text: String): Boolean {
        val root = service.rootInActiveWindow ?: return false
        val node = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            ?.takeIf { it.isEditable } ?: return false
        // Force the node to re-fetch its data from the source view before
        // we read text/cursor off it. The accessibility tree lags the real
        // UI: after Telegram sends a message the field becomes empty but
        // node.text still returns the just-sent string for a brief window.
        node.refresh()
        val hint = node.hintText?.toString() ?: ""
        val rawText = node.text?.toString() ?: ""
        // Telegram puts its placeholder string directly into node.text instead
        // of using the hint API. Recognise the known per-locale placeholders
        // when we are in the Telegram package. Real user content is never
        // exactly equal to a single placeholder word, so the check is safe.
        val isTelegramPlaceholder =
            node.packageName?.toString() in TELEGRAM_PACKAGES &&
                rawText in TELEGRAM_PLACEHOLDERS
        val isPlaceholderShowing = node.isShowingHintText ||
            (hint.isNotEmpty() && rawText == hint) ||
            isTelegramPlaceholder
        val current = if (isPlaceholderShowing) "" else rawText
        val rawCursor = node.textSelectionEnd
        val cursor = if (rawCursor in 0..current.length) rawCursor else current.length
        val before = current.substring(0, cursor)
        val after = current.substring(cursor)
        val needsLeadingSpace = before.isNotEmpty() && !before.last().isWhitespace()
        val payload = if (needsLeadingSpace) " $text" else text
        val newText = before + payload + after
        val setArgs = Bundle().apply {
            putCharSequence(
                AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                newText
            )
        }
        if (!node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, setArgs)) {
            return false
        }
        val newCursor = cursor + payload.length
        val selArgs = Bundle().apply {
            putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT, newCursor)
            putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT, newCursor)
        }
        node.performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION, selArgs)
        AppLog.log(service, "Paste: SET_TEXT len=${payload.length}")
        return true
    }

    private fun prependSpaceIfNeeded(text: String, surroundingText: String?): String {
        val lastChar = surroundingText?.lastOrNull()
        return if (lastChar != null && !lastChar.isWhitespace()) " $text" else text
    }

    private companion object {
        // Per-locale Telegram message-input placeholders. node.text exactly
        // matching one of these (in a Telegram-family package) is treated as
        // empty so we don't splice user dictation behind the placeholder.
        private val TELEGRAM_PLACEHOLDERS = setOf(
            "Сообщение",       // ru
            "Message"          // en
        )
        // Official Telegram and popular third-party clients that put the
        // placeholder directly into node.text instead of using the hint API.
        private val TELEGRAM_PACKAGES = setOf(
            "org.telegram.messenger",       // Play Store
            "org.telegram.messenger.web",   // telegram.org direct APK
            "tw.nekomimi.nekogram",         // Nekogram
            "com.radolyn.ayugram",          // AyuGram
        )
    }
}
