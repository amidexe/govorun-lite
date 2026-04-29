package com.govorun.lite.util

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.provider.Settings
import android.text.TextUtils
import com.govorun.lite.service.LiteAccessibilityService

/**
 * Whether [LiteAccessibilityService] is currently enabled in system settings.
 *
 * Uses the colon-separated [Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES]
 * string rather than [android.view.accessibility.AccessibilityManager], because
 * the latter also needs the service to have connected, which can race on first
 * toggle.
 */
object AccessibilityHelper {

    fun isLiteServiceEnabled(context: Context): Boolean {
        val expected = ComponentName(context, LiteAccessibilityService::class.java)
        val enabled = Settings.Secure.getString(
            context.contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        val splitter = TextUtils.SimpleStringSplitter(':')
        splitter.setString(enabled)
        while (splitter.hasNext()) {
            val component = ComponentName.unflattenFromString(splitter.next()) ?: continue
            if (component == expected) return true
        }
        return false
    }

    /**
     * Whether the user enabled the system accessibility Shortcut for our service.
     * Covers the navbar / floating a11y button (accessibility_button_targets) and
     * the volume-key shortcut (accessibility_shortcut_target_service). Either one
     * manifests as an unwanted extra floating icon on top of the bubble.
     */
    fun isLiteShortcutEnabled(context: Context): Boolean {
        val expected = ComponentName(context, LiteAccessibilityService::class.java)
        val keys = arrayOf(
            "accessibility_button_targets",
            "accessibility_shortcut_target_service",
        )
        for (key in keys) {
            val raw = Settings.Secure.getString(context.contentResolver, key) ?: continue
            val splitter = TextUtils.SimpleStringSplitter(':')
            splitter.setString(raw)
            while (splitter.hasNext()) {
                val component = ComponentName.unflattenFromString(splitter.next()) ?: continue
                if (component == expected) return true
            }
        }
        return false
    }

    /**
     * Builds an Intent that opens the system Accessibility Settings list
     * with our service highlighted. The Settings app reads two undocumented
     * but de-facto-stable extras (`:settings:fragment_args_key` and
     * `:settings:show_fragment_args`) and uses them to scroll the list to
     * the matching row and play a brief attention pulse. This is the same
     * mechanism Wi-Fi / Bluetooth tiles use to deep-link to a particular
     * network or device.
     *
     * Especially helpful on long Settings lists (Xiaomi/HyperOS,
     * Honor/MagicOS, Huawei) where our service might otherwise be lost
     * among 30+ accessibility entries.
     *
     * Caller picks how to start the activity:
     *  - Activity context → [openAccessibilitySettings] below
     *  - TileService → startActivityAndCollapse with PendingIntent (mandatory
     *    on Android 14+ due to background-activity-start restrictions)
     *
     * We deliberately DO NOT use `ACCESSIBILITY_DETAILS_SETTINGS` here: on
     * Android 13+ when the install came from a sideloaded APK, that intent
     * gets rewritten by the OS into the App Details page (with the "Allow
     * restricted settings" overflow) instead of the per-service details page
     * — so the button would bounce the user back to where they already were,
     * never reaching the toggle. ACTION_ACCESSIBILITY_SETTINGS + the
     * highlight extras gives the same deep-link feel without the hijack.
     */
    fun buildOpenAccessibilityIntent(context: Context): Intent {
        val componentName = android.content.ComponentName(
            context, "com.govorun.lite.service.LiteAccessibilityService"
        ).flattenToString()
        val highlightBundle = android.os.Bundle().apply {
            putString(":settings:fragment_args_key", componentName)
        }
        return Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            putExtra(":settings:fragment_args_key", componentName)
            putExtra(":settings:show_fragment_args", highlightBundle)
        }
    }

    /**
     * Convenience wrapper for Activity callers (MainActivity card, Settings,
     * Onboarding). Builds the highlight intent and starts it via plain
     * startActivity. TileService callers should use [buildOpenAccessibilityIntent]
     * directly + startActivityAndCollapse.
     */
    fun openAccessibilitySettings(context: Context) {
        try {
            context.startActivity(buildOpenAccessibilityIntent(context))
        } catch (_: Exception) {
            // Settings app missing/locked — nothing else to do.
        }
    }
}
