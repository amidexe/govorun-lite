package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.os.Bundle
import android.view.accessibility.AccessibilityNodeInfo
import com.govorun.lite.stats.AndroidStatsRecorder
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.transcriber.Dictionary
import com.govorun.lite.util.AppLog

/**
 * Inserts recognised text into whatever editable is currently focused.
 *
 * Three layers of insertion attempt:
 *  1) Android 13+ — `commitText` through [InputMethodBridge].
 *  2) Post-commit verification — re-read text before cursor; if the field
 *     does not end with what we just committed, the IC is "zombie" (some
 *     apps recycle their EditText, leaving us with a connection that
 *     accepts commits silently but never delivers them to the visible
 *     view). The simpler probe `getSurroundingText==null` catches some
 *     zombie variants but not all — Telegram has been observed accepting
 *     commits while `getSurroundingText` returned a non-null empty value.
 *  3) Android 12 and zombie-connection fallback — find the focused editable
 *     node fresh in the accessibility tree and write via ACTION_SET_TEXT.
 *
 * Trade-off accepted in current design: post-commit verification can
 * occasionally false-positive (verification reads stale snapshot, thinks
 * commit failed when it actually landed) leading to a possible duplicate
 * write via the fallback path. Recovering through the alternative path is
 * preferable to silently dropping recognised text.
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
internal class TextInserter(
    private val service: AccessibilityService,
    private val inputMethodBridge: InputMethodBridge,
    private val onConfirmed: () -> Unit = {},
) {

    fun insert(rawText: String): Boolean {
        if (rawText.isBlank()) return false
        val replaced = Dictionary.applyReplacements(service, rawText)
        // Before the attempts: the text that "all paths failed" would drop stays copyable.
        LastDictation.remember(service, replaced)
        if (replaced != rawText) {
            AppLog.log(service, "Dictionary applied (raw=${rawText.length} → result=${replaced.length})")
        }

        if (inputMethodBridge.tryCommit(replaced)) {
            onConfirmed()
            AndroidStatsRecorder.create(service).addWords(StatsStore.countWords(replaced))
            return true
        }
        // commitText didn't land (IC null or zombie). Walk the accessibility
        // tree and write directly via ACTION_SET_TEXT.
        if (setTextOnFocusedEditable(replaced)) {
            onConfirmed()
            AndroidStatsRecorder.create(service).addWords(StatsStore.countWords(replaced))
            return true
        } else {
            AppLog.log(service, "Paste: all paths failed — dropped len=${replaced.length}")
            return false
        }
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
        val plan = TextInsertionPlan.forAccessibilityNode(
            dictatedText = text,
            rawText = rawText,
            hint = hint,
            isShowingHintText = node.isShowingHintText,
            isTelegramPlaceholder = isTelegramPlaceholder,
            rawCursor = node.textSelectionEnd,
        )
        val setArgs = Bundle().apply {
            putCharSequence(
                AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                plan.newText
            )
        }
        if (!node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, setArgs)) {
            return false
        }
        val selArgs = Bundle().apply {
            putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT, plan.newCursor)
            putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT, plan.newCursor)
        }
        node.performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION, selArgs)
        AppLog.log(service, "Paste: SET_TEXT len=${plan.payload.length}")
        return true
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
            "org.telegram.plus",            // Plus Messenger
            "nu.gpu.nagram",               // Nagram
            "nu.gpu.nagramx",              // NagramX / NagramXF
        )
    }
}

/** Pure text planning shared by the API-33 and Android-12 insertion paths. */
internal object TextInsertionPlan {
    data class Result(
        val newText: String,
        val newCursor: Int,
        val payload: String,
    )

    fun prependSpaceIfNeeded(text: String, surroundingText: String?): String {
        val lastChar = surroundingText?.lastOrNull()
        val startsWithWhitespace = text.firstOrNull()?.isWhitespace() == true
        return if (lastChar != null && !lastChar.isWhitespace() && !startsWithWhitespace) {
            " $text"
        } else {
            text
        }
    }

    fun forAccessibilityNode(
        dictatedText: String,
        rawText: String,
        hint: String,
        isShowingHintText: Boolean,
        isTelegramPlaceholder: Boolean,
        rawCursor: Int,
    ): Result {
        val isPlaceholderShowing = isShowingHintText ||
            (hint.isNotEmpty() && rawText == hint) ||
            isTelegramPlaceholder
        val current = if (isPlaceholderShowing) "" else rawText
        val cursor = if (rawCursor in 0..current.length) rawCursor else current.length
        val before = current.substring(0, cursor)
        val after = current.substring(cursor)
        val payload = prependSpaceIfNeeded(dictatedText, before)
        return Result(
            newText = before + payload + after,
            newCursor = cursor + payload.length,
            payload = payload,
        )
    }
}
