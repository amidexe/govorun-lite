package com.govorun.lite.overlay

import android.content.Context
import com.govorun.lite.util.StorageKeys
import com.govorun.lite.util.Prefs
import kotlin.math.roundToInt

internal object BubblePlacement {
    fun hasCustomMargin(context: Context): Boolean =
        context.getSharedPreferences(StorageKeys.PREFS_FILE, Context.MODE_PRIVATE)
            .contains(StorageKeys.Key.BUBBLE_EDGE_MARGIN)

    fun horizontalOffsetPx(context: Context): Int = bubbleHorizontalOffsetPx(
        marginDp = if (hasCustomMargin(context)) {
            Prefs.getBubbleEdgeMargin(context).coerceAtMost(Prefs.bubbleEdgeMarginMax(context))
        } else null,
        density = context.resources.displayMetrics.density,
        sizeScale = Prefs.getBubbleSize(context),
    )
}

internal fun bubbleHorizontalOffsetPx(marginDp: Int?, density: Float, sizeScale: Float): Int {
    if (marginDp != null) return (marginDp * density).toInt()
    // 1.0.15 had no margin preference: its window was inset by 16 physical px.
    // Keep the bird's centre despite the narrower v2 idle surface. An explicit
    // v2 margin (including zero) retains its existing meaning and position.
    val bubbleSize = (BUBBLE_TOUCH_BASE_DP * density * sizeScale).toInt()
    val legacyExtent = bubbleSize + (bubbleSize * 0.6f).toInt()
    val currentExtent = bubbleTouchExtentPx(density, sizeScale)
    return (16f + (legacyExtent - currentExtent) / 2f).roundToInt()
}
