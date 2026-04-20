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
     * Opens the top-level system Accessibility Settings screen (the list of
     * all accessibility services).
     *
     * We deliberately DO NOT use `ACCESSIBILITY_DETAILS_SETTINGS` here: on
     * Android 13+ when the install came from a sideloaded APK, that intent
     * gets rewritten by the OS into the App Details page (with the "Allow
     * restricted settings" overflow) instead of the per-service details page
     * — so the button would bounce the user back to where they already were,
     * never reaching the toggle. The plain ACTION_ACCESSIBILITY_SETTINGS is
     * boring but always lands on the actual list.
     */
    fun openAccessibilitySettings(context: Context) {
        val intent = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(intent)
        } catch (_: Exception) {
            // Settings app missing/locked — nothing else to do.
        }
    }
}
