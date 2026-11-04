package com.govorun.lite.service

import android.app.PendingIntent
import android.os.Build
import android.graphics.drawable.Icon
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import com.govorun.lite.R

/** Pins the floating button without the keyboard; a second tap restores its normal rules.
 * The missing AccessibilityService needs the user's permission in system settings. */
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
            service.toggleAlwaysShow()
            refreshTileState()
        }
    }

    private fun openAccessibilitySettings() {
        val intent = com.govorun.lite.util.AccessibilityHelper
            .buildOpenAccessibilityIntent(this)
        // TileService → startActivityAndCollapse is mandatory on Android
        // 14+ (background-activity-start restrictions); plain startActivity
        // silently no-ops there.
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
        tile.label = getString(R.string.tile_label)
        tile.icon = Icon.createWithResource(this, R.drawable.ic_bird_24)
        tile.contentDescription = "${getString(R.string.app_name)}: ${tile.label}"
        when {
            service == null -> {
                tile.state = Tile.STATE_INACTIVE
                tile.subtitle = getString(R.string.tile_subtitle_service_off)
            }
            // ACTIVE = always-show mode (bubble stays even without IME).
            // INACTIVE = standard mode (bubble follows the keyboard).
            service.isAlwaysShowEnabled() -> {
                tile.state = Tile.STATE_ACTIVE
                tile.subtitle = getString(R.string.tile_subtitle_always)
            }
            else -> {
                tile.state = Tile.STATE_INACTIVE
                tile.subtitle = getString(R.string.tile_subtitle_default)
            }
        }
        tile.stateDescription = tile.subtitle
        tile.updateTile()
    }
}
