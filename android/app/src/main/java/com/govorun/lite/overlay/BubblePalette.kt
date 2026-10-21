package com.govorun.lite.overlay

import android.content.Context
import android.graphics.Paint
import android.util.TypedValue
import androidx.core.content.ContextCompat
import androidx.core.graphics.ColorUtils

internal class BubblePalette(
    private val context: Context,
    private val invalidate: () -> Unit,
) {
    private var idleAlphaFraction = DEFAULT_IDLE_ALPHA
    private var recordingAlphaFraction = DEFAULT_RECORDING_ALPHA
    private var birdColor = BirdColors.DEFAULT

    private var onSurfaceColor = themeColor(com.google.android.material.R.attr.colorOnSurface, 0xFF1D1B20.toInt())
    private var surfaceColor = themeColor(com.google.android.material.R.attr.colorSurfaceContainerHigh, FALLBACK_SURFACE)

    // Resolved from the bird colour and theme by [resolve].
    private var fillRgb = 0
    private var onFill = 0

    val basePaint = fillPaint(0)
    val legacyRecordingPaint = fillPaint(0)
    val railPaint = fillPaint(0)
    val legacyRecordingHaloPaint = fillPaint(0)
    val idleHaloPaint = fillPaint(0)
    val dotPaint = fillPaint(0)

    init {
        resolve()
    }

    fun setIdlePulseColor(color: Int) {
        idleHaloPaint.color = (color and RGB) or OPAQUE
        invalidate()
    }

    fun setIdleAlpha(alpha: Float) {
        idleAlphaFraction = alpha.coerceIn(0f, 1f)
        resolve()
    }

    fun setRecordingAlpha(alpha: Float) {
        recordingAlphaFraction = alpha.coerceIn(0f, 1f)
        resolve()
    }

    fun setBirdColor(id: String) {
        birdColor = BirdColors.normalize(id)
        resolve()
    }

    fun idleBirdColor(): Int = foregroundOn(composeIdleFill())

    fun recordingForeground(): Int = foregroundOn(composeRecordingFill())

    /**
     * Glyphs stay opaque whatever the fill transparency: the colour drawn on the
     * fill, or onSurface once the fill is too faint for it to read.
     */
    private fun foregroundOn(fill: Int): Int {
        val plain = onSurfaceColor or OPAQUE
        val backdrop = ColorUtils.compositeColors(fill, surfaceColor or OPAQUE)
        return if (ColorUtils.calculateContrast(onFill, backdrop) >= ColorUtils.calculateContrast(plain, backdrop)) {
            onFill
        } else {
            plain
        }
    }

    /**
     * One fill for the whole rail. [recordingShare] 0 is the idle surface, 1 the
     * recording fill; blending both ways (alpha included) means the rail never
     * snaps between two backgrounds when a recording starts or ends.
     */
    fun railFill(recordingShare: Float): Int =
        ColorUtils.blendARGB(composeIdleFill(), composeRecordingFill(), recordingShare.coerceIn(0f, 1f))

    fun railForeground(recordingShare: Float): Int =
        ColorUtils.blendARGB(idleBirdColor(), recordingForeground(), recordingShare.coerceIn(0f, 1f))

    fun setThemeColors(colors: BubbleThemeColors) {
        onSurfaceColor = colors.onSurface
        surfaceColor = colors.surfaceContainerHigh
        resolve()
    }

    fun refresh() {
        onSurfaceColor = themeColor(com.google.android.material.R.attr.colorOnSurface, onSurfaceColor)
        surfaceColor = themeColor(com.google.android.material.R.attr.colorSurfaceContainerHigh, surfaceColor)
        resolve()
    }

    /** One colour for every state; only the idle and recording transparency differ. */
    private fun resolve() {
        val tone = BirdColors.tone(context, birdColor)
        fillRgb = tone.fill and RGB
        onFill = tone.onFill or OPAQUE
        basePaint.color = composeIdleFill()
        legacyRecordingPaint.color = composeRecordingFill()
        legacyRecordingHaloPaint.color = RECORDING_HALO_ALPHA or fillRgb
        idleHaloPaint.color = fillRgb or OPAQUE
        dotPaint.color = idleBirdColor()
        invalidate()
    }

    private fun composeRecordingFill(): Int =
        ((recordingAlphaFraction * 255).toInt() shl 24) or fillRgb

    private fun composeIdleFill(): Int =
        ((idleAlphaFraction * 255).toInt() shl 24) or fillRgb

    private fun themeColor(attribute: Int, fallback: Int): Int {
        val value = TypedValue()
        val resolved = context.theme.resolveAttribute(attribute, value, true)
        return if (!resolved) fallback
        else if (value.resourceId != 0) ContextCompat.getColor(context, value.resourceId)
        else value.data
    }

    private fun fillPaint(color: Int) = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        this.color = color
        style = Paint.Style.FILL
    }

    private companion object {
        const val RGB = 0x00FFFFFF
        const val OPAQUE = 0xFF000000.toInt()
        const val RECORDING_HALO_ALPHA = 0x40000000
        const val FALLBACK_SURFACE = 0xFFECE6F0.toInt()
        const val DEFAULT_IDLE_ALPHA = 9f / 14f
        const val DEFAULT_RECORDING_ALPHA = 1f
    }
}
