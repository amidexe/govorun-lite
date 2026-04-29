package com.govorun.lite.service

import android.app.PendingIntent
import android.content.Intent
import android.os.Build
import android.provider.Settings
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import com.govorun.lite.R

/**
 * Quick Settings Tile — three behaviours rolled into one tap target:
 *
 *  1. Service alive + bubble visible → tap hides the bubble (manual override).
 *  2. Service alive + bubble hidden  → tap shows it again.
 *  3. Service dead (killed by OEM / disabled by user) → tap routes the user
 *     to system Accessibility settings, where they can flip our service
 *     back on. We can't programmatically re-enable an AccessibilityService
 *     for security reasons, so this is the best a tile can do.
 *
 * Useful for: people who find the bubble distracting in some apps and want
 * a quick on/off without digging into Settings; users on aggressive OEMs
 * (Tecno HiOS, OxygenOS) where the service occasionally dies and the tile
 * gives a one-tap path back into the Accessibility screen instead of
 * opening our app and hunting for the right row.
 */
class LiteTileService : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        refreshTileState()
    }

    override fun onClick() {
        super.onClick()
        val service = LiteAccessibilityService.instance
        if (service == null) {
            openAccessibilitySettings()
        } else {
            service.toggleBubbleVisibility()
            refreshTileState()
        }
    }

    private fun openAccessibilitySettings() {
        val intent = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        // Android 14+ requires startActivityAndCollapse to take a
        // PendingIntent; the Intent overload is deprecated and throws
        // UnsupportedOperationException on newer system images.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            val pending = PendingIntent.getActivity(
                this, 0, intent, PendingIntent.FLAG_IMMUTABLE
            )
            startActivityAndCollapse(pending)
        } else {
            @Suppress("DEPRECATION", "StartActivityAndCollapseDeprecated")
            startActivityAndCollapse(intent)
        }
    }

    private fun refreshTileState() {
        val tile = qsTile ?: return
        val service = LiteAccessibilityService.instance
        when {
            service == null -> {
                tile.state = Tile.STATE_INACTIVE
                tile.subtitle = getString(R.string.tile_subtitle_service_off)
            }
            // Reflects the user's intent (manualHide flag), not whether the
            // bubble is currently painted. Tapping the tile with no IME up
            // still flips state — the user sees their tap "took effect" in
            // the shade, even though no bubble appears until they open a
            // text field.
            service.isBubbleEnabled() -> {
                tile.state = Tile.STATE_ACTIVE
                tile.subtitle = getString(R.string.tile_subtitle_enabled)
            }
            else -> {
                tile.state = Tile.STATE_INACTIVE
                tile.subtitle = getString(R.string.tile_subtitle_disabled)
            }
        }
        tile.updateTile()
    }
}
