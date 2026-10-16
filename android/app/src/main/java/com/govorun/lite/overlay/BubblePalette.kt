package com.govorun.lite.overlay

import android.content.Context
import android.graphics.Paint
import android.util.TypedValue
import androidx.core.content.ContextCompat

internal class BubblePalette(
    private val context: Context,
    private val invalidate: () -> Unit,
) {
    private var idleAlphaFraction = DEFAULT_IDLE_ALPHA
    private var idleFillRgb = themeRgb(
        com.google.android.material.R.attr.colorPrimaryContainer,
        FALLBACK_IDLE_FILL,
    )
    private var idleBirdTint = themeRgb(
        com.google.android.material.R.attr.colorOnPrimaryContainer,
        FALLBACK_IDLE_TINT,
    ) or OPAQUE

    private var onSurfaceColor = themeColor(com.google.android.material.R.attr.colorOnSurface, 0xFF1D1B20.toInt())

    val basePaint = fillPaint(composeIdleFill())
    val legacyRecordingPaint = fillPaint(LEGACY_RECORDING_FILL)
    val legacyRecordingHaloPaint = fillPaint(0x40E53935)
    val idleHaloPaint = fillPaint(haloBaseColor())
    val dotPaint = fillPaint(idleBirdColor())

    fun setIdlePulseColor(color: Int) {
        idleHaloPaint.color = (color and RGB) or OPAQUE
        invalidate()
    }

    fun setIdleAlpha(alpha: Float) {
        idleAlphaFraction = alpha.coerceIn(0f, 1f)
        basePaint.color = composeIdleFill()
        invalidate()
    }

    /**
     * The circle and expanded control share primaryContainer and its foreground.
     * As their single background fades, keep every glyph opaque and move its
     * colour towards onSurface for the content underneath.
     */
    fun idleBirdColor(): Int = lerpColor(onSurfaceColor, idleBirdTint, idleAlphaFraction)

    fun setThemeColors(colors: BubbleThemeColors) {
        idleFillRgb = colors.primaryContainer and RGB
        idleBirdTint = colors.onPrimaryContainer or OPAQUE
        onSurfaceColor = colors.onSurface
        basePaint.color = composeIdleFill()
        idleHaloPaint.color = colors.primaryContainer
        dotPaint.color = idleBirdColor()
        invalidate()
    }

    fun refresh() {
        idleFillRgb = themeRgb(
            com.google.android.material.R.attr.colorPrimaryContainer,
            FALLBACK_IDLE_FILL,
        )
        idleBirdTint = themeRgb(
            com.google.android.material.R.attr.colorOnPrimaryContainer,
            FALLBACK_IDLE_TINT,
        ) or OPAQUE
        onSurfaceColor = themeColor(com.google.android.material.R.attr.colorOnSurface, onSurfaceColor)
        basePaint.color = composeIdleFill()
        idleHaloPaint.color = haloBaseColor()
        dotPaint.color = idleBirdColor()
        invalidate()
    }

    private fun haloBaseColor() =
        themeRgb(com.google.android.material.R.attr.colorPrimaryContainer, FALLBACK_IDLE_FILL) or OPAQUE

    private fun composeIdleFill(): Int {
        val alpha = (idleAlphaFraction.coerceIn(0f, 1f) * 255).toInt()
        return (alpha shl 24) or (idleFillRgb and RGB)
    }

    private fun themeRgb(attribute: Int, fallback: Int) = themeColor(attribute, fallback) and RGB

    private fun themeColor(attribute: Int, fallback: Int): Int {
        val value = TypedValue()
        val resolved = context.theme.resolveAttribute(attribute, value, true)
        return if (!resolved) fallback
        else if (value.resourceId != 0) ContextCompat.getColor(context, value.resourceId)
        else value.data
    }

    private fun lerpColor(from: Int, to: Int, fraction: Float): Int {
        val progress = fraction.coerceIn(0f, 1f)
        fun channel(shift: Int): Int {
            val start = (from shr shift) and 0xFF
            val end = (to shr shift) and 0xFF
            return start + ((end - start) * progress).toInt()
        }
        return (OPAQUE) or
            (channel(16) shl 16) or
            (channel(8) shl 8) or
            channel(0)
    }

    private fun fillPaint(color: Int) = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        this.color = color
        style = Paint.Style.FILL
    }

    private companion object {
        const val RGB = 0x00FFFFFF
        const val OPAQUE = 0xFF000000.toInt()
        const val FALLBACK_IDLE_FILL = 0x80404040.toInt()
        const val FALLBACK_IDLE_TINT = 0xFFFFFFFF.toInt()
        const val LEGACY_RECORDING_FILL = 0xFFE53935.toInt()
        const val DEFAULT_IDLE_ALPHA = 12f / 14f
    }
}
