package com.govorun.lite.util

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings

/**
 * Xiaomi, Redmi and POCO phones (MIUI, HyperOS) unload the accessibility
 * process after heavy apps unless «Автозапуск» is allowed for Govorun. On
 * 4PDA that switch was the only thing that kept the bird alive for testers.
 */
object XiaomiShell {

    val detected: Boolean = listOf(Build.MANUFACTURER, Build.BRAND)
        .any { it.lowercase() in setOf("xiaomi", "redmi", "poco") }

    /** Opens the MIUI autostart list, or the app details page when it is missing. */
    fun openAutostart(context: Context) {
        val autostart = Intent().setComponent(
            ComponentName(
                "com.miui.securitycenter",
                "com.miui.permcenter.autostart.AutoStartManagementActivity",
            ),
        ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val details = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
            .setData(Uri.fromParts("package", context.packageName, null))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        runCatching { context.startActivity(autostart) }
            .recoverCatching { context.startActivity(details) }
    }
}
