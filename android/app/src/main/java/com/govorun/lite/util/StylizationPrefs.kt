package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.storage.GovorunPreferences

internal object StylizationPrefs {
    private const val FILE_NAME = StorageKeys.PREFS_FILE
    private const val KEY_ENABLED = StorageKeys.Key.STYLIZATION_ENABLED
    private const val KEY_STYLE = StorageKeys.Key.STYLIZATION_STYLE
    private const val KEY_CONSENT_VERSION = StorageKeys.Key.STYLIZATION_CONSENT_VERSION
    private const val KEY_CONSENT_AT = StorageKeys.Key.STYLIZATION_CONSENT_AT
    private const val KEY_EMOJI_MODE = "stylization_emoji_mode"
    private const val KEY_CUSTOM_INSTRUCTION = "stylization_custom_instruction"

    fun isEnabled(context: Context) = prefs(context).getBoolean(KEY_ENABLED, false)

    fun setEnabled(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_ENABLED, enabled).apply()
    }

    fun style(context: Context): String = prefs(context).getString(KEY_STYLE, "formal") ?: "formal"

    fun setStyle(context: Context, styleId: String) {
        prefs(context).edit()
            .putString(KEY_STYLE, styleId)
            .apply()
    }

    fun customInstruction(context: Context): String =
        prefs(context).getString(KEY_CUSTOM_INSTRUCTION, "") ?: ""

    fun setCustomInstruction(context: Context, instruction: String) {
        prefs(context).edit()
            .putString(KEY_CUSTOM_INSTRUCTION, instruction)
            .apply()
    }

    fun emojiMode(context: Context): EmojiMode =
        EmojiMode.fromApiId(prefs(context).getString(KEY_EMOJI_MODE, null))

    fun setEmojiMode(context: Context, mode: EmojiMode) {
        prefs(context).edit()
            .putString(KEY_EMOJI_MODE, mode.apiId)
            .apply()
    }

    fun hasConsent(context: Context, version: Int) =
        prefs(context).getInt(KEY_CONSENT_VERSION, 0) == version

    fun consentVersion(context: Context) = prefs(context).getInt(KEY_CONSENT_VERSION, 0)

    fun dismissTermsUpdate(context: Context) {
        // Declining activation is not withdrawal of an existing receipt. An old
        // receipt still fails hasConsent(currentVersion) after a terms change.
        prefs(context).edit()
            .putBoolean(KEY_ENABLED, false)
            .apply()
    }

    fun acceptTerms(context: Context, version: Int, acceptedAtMillis: Long) {
        prefs(context).edit()
            .putInt(KEY_CONSENT_VERSION, version)
            .putLong(KEY_CONSENT_AT, acceptedAtMillis)
            .apply()
    }

    fun consentAt(context: Context) = prefs(context).getLong(KEY_CONSENT_AT, 0L)

    private fun prefs(context: Context) =
        GovorunPreferences.of(context, FILE_NAME)
}
