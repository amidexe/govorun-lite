package com.govorun.lite.service

import android.app.PendingIntent
import android.content.Intent
import android.os.Build
import android.provider.Settings
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import com.govorun.lite.R

/**
 * Quick Settings Tile that silences Говорун — bubble stops appearing even
 * when an IME is up. Symmetric toggle: one tap silences, one tap wakes.
 * Doesn't disable the AccessibilityService itself, just hides the bubble
 * via the manualSilence flag on the service.
 *
 * Useful at night, in meetings, in libraries — anywhere the user wants
 * voice input out of the way without going through Settings.
 *
 * Lives next to [LiteTileService] (always-show toggle) in the shade as a
 * separate tile with a different icon, so the user can drop both into
 * Quick Settings if they want both controls.
 */
class LiteSilenceTileService : TileService() {

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
            service.toggleSilence()
            refreshTileState()
        }
    }

    private fun openAccessibilitySettings() {
        val intent = com.govorun.lite.util.AccessibilityHelper
            .buildOpenAccessibilityIntent(this)
        // TileService → startActivityAndCollapse is mandatory on Android
        // 14+ (background-activity-start restrictions).
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
            // Inverted: power-icon semantics match the Wi-Fi/Bluetooth
            // pattern — ACTIVE means the function is ON (bubble works),
            // INACTIVE means it's been temporarily silenced. Without this
            // inversion the icon felt backwards: lit when the bubble was
            // hidden, off when it was working.
            !service.isSilenceEnabled() -> {
                tile.state = Tile.STATE_ACTIVE
                tile.subtitle = getString(R.string.tile_silence_subtitle_off)
            }
            else -> {
                tile.state = Tile.STATE_INACTIVE
                tile.subtitle = getString(R.string.tile_silence_subtitle_on)
            }
        }
        tile.updateTile()
    }
}
