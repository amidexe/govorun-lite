package com.govorun.lite.service

import android.text.InputType
import android.view.inputmethod.EditorInfo

/**
 * Filters that decide whether the bubble should be visible based on the
 * IME's current input editor info. Lives outside [LiteAccessibilityService]
 * to keep that class focused on lifecycle and event dispatch — pure
 * inspection helpers don't need to be members.
 *
 * Extracted in 1.0.6 when password-field detection pushed the service
 * past the 500-line ceiling. Future filters (e.g. URL fields, sensitive
 * search boxes) belong here too.
 */
internal object InputFieldFilter {

    /**
     * True if the IME is currently bound to a password-style field.
     * Reads inputType from the bound editor info — the canonical signal
     * that every Android IME uses to render dots-instead-of-text and
     * suppress autocorrect. Banking PIN screens, login forms, password
     * managers and 2FA entries all set one of these variation flags.
     *
     * Returns false on null inputs (no IME bound, no editor info) — the
     * safe default is "not a password", so we'd rather leak a bubble
     * appearance than miss showing it on a regular field.
     */
    fun isPasswordField(inputMethod: LiteAccessibilityInputMethod?): Boolean {
        val info = inputMethod?.currentInputEditorInfo
            ?: inputMethod?.lastEditorInfo
            ?: return false
        val variation = info.inputType and InputType.TYPE_MASK_VARIATION
        val klass = info.inputType and InputType.TYPE_MASK_CLASS
        return when (klass) {
            InputType.TYPE_CLASS_TEXT -> variation == InputType.TYPE_TEXT_VARIATION_PASSWORD ||
                variation == InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD ||
                variation == InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD
            InputType.TYPE_CLASS_NUMBER -> variation == InputType.TYPE_NUMBER_VARIATION_PASSWORD
            else -> false
        }
    }

    /**
     * True if the IME is bound to a search-style or address-bar-style
     * input — the user is typing to find/navigate, not to compose. These
     * places usually feel cluttered with our bubble overlapping the
     * keyboard. Catch four distinct signals:
     *
     *   - imeOptions IME_ACTION_SEARCH — explicit "Search" action key
     *     (Play Store search, system search, Telegram chat-list search,
     *     in-app search of all kinds)
     *   - imeOptions IME_ACTION_GO — explicit "Go" action key (Chrome
     *     omnibox and other browser address bars, "open URL" prompts)
     *   - inputType TYPE_TEXT_VARIATION_URI — declared URL input
     *     (browser address bars even when imeOptions doesn't say GO,
     *     URL prompts in apps)
     *   - inputType TYPE_TEXT_VARIATION_FILTER — declared filter input
     *     (filterable list searches, dropdown filters)
     *
     * Power users who do want voice search can enable the «Говорун:
     * всегда» Quick Settings tile to bypass this filter for the whole
     * session — it's the explicit "voice everywhere" override.
     */
    fun isSearchField(inputMethod: LiteAccessibilityInputMethod?): Boolean {
        val info = inputMethod?.currentInputEditorInfo
            ?: inputMethod?.lastEditorInfo
            ?: return false
        val action = info.imeOptions and EditorInfo.IME_MASK_ACTION
        if (action == EditorInfo.IME_ACTION_SEARCH ||
            action == EditorInfo.IME_ACTION_GO
        ) return true
        val variation = info.inputType and InputType.TYPE_MASK_VARIATION
        val klass = info.inputType and InputType.TYPE_MASK_CLASS
        return klass == InputType.TYPE_CLASS_TEXT &&
            (variation == InputType.TYPE_TEXT_VARIATION_FILTER ||
                variation == InputType.TYPE_TEXT_VARIATION_URI)
        // Detection is intentionally limited to the two language-independent
        // signals above (IME action, input type variation). A previous
        // version also matched the field's hint text against a regex of
        // "поиск|найти|search|find" — that caught Telegram's chat-list
        // search but required reading user-visible field content. Removed
        // for a clean privacy story: with this implementation we only ever
        // read technical bit flags from EditorInfo, never any text shown
        // to the user. Trade-off: Telegram chat-list search and other
        // fields with no IME_ACTION_SEARCH declared get the bubble.
    }
}
