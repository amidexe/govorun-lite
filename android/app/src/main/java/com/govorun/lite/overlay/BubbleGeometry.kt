package com.govorun.lite.overlay

import android.graphics.RectF
import kotlin.math.PI
import kotlin.math.max
import kotlin.math.sin

internal data class ContextualGeometry(
    val centerY: Float,
    val actionRadius: Float,
    val anchorX: Float,
    val cancelX: Float,
    val dotsX: Float,
    val rail: RectF,
)

internal fun contextualAnchorCenter(
    width: Int,
    idleExtent: Int,
    anchoredLeft: Boolean,
): Float = if (anchoredLeft) idleExtent / 2f else width - idleExtent / 2f

internal fun fallbackShakeOffset(progress: Float, amplitude: Float): Float {
    val clamped = progress.coerceIn(0f, 1f)
    return sin(clamped * PI.toFloat() * 4f) * (1f - clamped) * amplitude
}

internal fun processingControlWidthDp(previousWasHold: Boolean): Float =
    if (previousWasHold) 124f else 152f

internal data class ExclusiveIconAlphas(
    val outgoing: Float,
    val incoming: Float,
)

/**
 * Changes two unrelated glyphs without ever painting both in the same frame.
 * The outgoing glyph fades to a low-opacity hand-off, then the incoming glyph
 * fades up from the same opacity.
 */
internal fun exclusiveIconAlphas(
    progress: Float,
    switchAt: Float = 0.42f,
): ExclusiveIconAlphas {
    val clamped = progress.coerceIn(0f, 1f)
    val split = switchAt.coerceIn(0.01f, 0.99f)
    val minimumVisibleAlpha = 0.18f
    return if (clamped < split) {
        ExclusiveIconAlphas(
            outgoing = 1f - (1f - minimumVisibleAlpha) * (clamped / split),
            incoming = 0f,
        )
    } else {
        ExclusiveIconAlphas(
            outgoing = 0f,
            incoming = minimumVisibleAlpha +
                (1f - minimumVisibleAlpha) * ((clamped - split) / (1f - split)),
        )
    }
}

internal fun buildContextualGeometry(
    width: Int,
    height: Int,
    idleExtent: Int,
    bubbleSize: Int,
    anchoredLeft: Boolean,
    finalHeight: Float,
    reveal: Float,
    innerPadding: Float,
): ContextualGeometry {
    val centerY = height / 2f
    val railHeight = bubbleSize + (finalHeight - bubbleSize) * reveal
    val radius = railHeight / 2f
    val anchor = contextualAnchorCenter(width, idleExtent, anchoredLeft)
    val clampedReveal = reveal.coerceIn(0f, 1f)
    val rail = if (anchoredLeft) {
        val collapsedRight = anchor + radius
        val expandedRight = max(collapsedRight, width - innerPadding)
        RectF(
            anchor - radius,
            centerY - radius,
            collapsedRight + (expandedRight - collapsedRight) * clampedReveal,
            centerY + radius,
        )
    } else {
        val collapsedLeft = anchor - radius
        val expandedLeft = minOf(collapsedLeft, innerPadding)
        RectF(
            collapsedLeft + (expandedLeft - collapsedLeft) * clampedReveal,
            centerY - radius,
            anchor + radius,
            centerY + radius,
        )
    }
    val expandedCancel = if (anchoredLeft) {
        width - innerPadding - radius * 0.78f
    } else {
        innerPadding + radius * 0.78f
    }
    val cancel = anchor + (expandedCancel - anchor) * clampedReveal
    return ContextualGeometry(
        centerY = centerY,
        actionRadius = radius,
        anchorX = anchor,
        cancelX = cancel,
        dotsX = (anchor + cancel) / 2f,
        rail = rail,
    )
}

/**
 * Places a variable-width control inside a stable overlay surface.
 *
 * Keeping [windowWidth] unchanged prevents WindowManager from moving the
 * surface between the idle, recording and processing frames. Only the rail
 * grows towards the free side of the screen.
 */
internal fun buildAnchoredContextualGeometry(
    windowWidth: Int,
    controlWidth: Int,
    height: Int,
    idleExtent: Int,
    bubbleSize: Int,
    anchoredLeft: Boolean,
    finalHeight: Float,
    reveal: Float,
    innerPadding: Float,
): ContextualGeometry {
    val boundedControlWidth = controlWidth.coerceIn(idleExtent, windowWidth)
    val offsetX = if (anchoredLeft) 0f else (windowWidth - boundedControlWidth).toFloat()
    val local = buildContextualGeometry(
        width = boundedControlWidth,
        height = height,
        idleExtent = idleExtent,
        bubbleSize = bubbleSize,
        anchoredLeft = anchoredLeft,
        finalHeight = finalHeight,
        reveal = reveal,
        innerPadding = innerPadding,
    )
    return local.copy(
        anchorX = local.anchorX + offsetX,
        cancelX = local.cancelX + offsetX,
        dotsX = local.dotsX + offsetX,
        rail = RectF(
            local.rail.left + offsetX,
            local.rail.top,
            local.rail.right + offsetX,
            local.rail.bottom,
        ),
    )
}

/** Android's minimum touch target (Material and accessibility guidance). */
internal const val MIN_TOUCH_TARGET_DP = 48f

/** Bird circle diameter at 100 %: the rail's height, so the circle only stretches sideways. */
internal const val BUBBLE_BASE_DP = 48f

/** The earlier circle size the idle touch window is still measured from. */
internal const val BUBBLE_TOUCH_BASE_DP = 56f

/**
 * Idle touch window: the earlier 56 dp circle plus 40 %, rounded exactly as before,
 * so the slimmer circle is as easy to hit and sits at the same place.
 */
internal fun bubbleTouchExtentPx(density: Float, sizeScale: Float): Int {
    val base = (BUBBLE_TOUCH_BASE_DP * density * sizeScale).toInt()
    return base + (base * 0.4f).toInt()
}

/**
 * True when [x] belongs to the cancel action of the confirmation control.
 *
 * Cancel is the secondary action, so it gets only the minimum touch target
 * centred on its icon; everything else, including the dots between the two
 * icons, confirms. The target never grows past the rail, so on a narrow
 * control it stays at the far edge instead of eating the confirm side.
 */
internal fun isCancelTarget(
    x: Float,
    cancelX: Float,
    rail: RectF,
    anchoredLeft: Boolean,
    minTargetPx: Float,
): Boolean {
    val half = minTargetPx / 2f
    return if (anchoredLeft) {
        x >= minOf(cancelX - half, rail.right - minTargetPx)
    } else {
        x <= maxOf(cancelX + half, rail.left + minTargetPx)
    }
}
