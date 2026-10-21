package com.govorun.lite.overlay

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.drawable.Drawable
import kotlin.math.PI
import kotlin.math.sin

internal fun drawDrawable(
    canvas: Canvas,
    drawable: Drawable?,
    centerX: Float,
    centerY: Float,
    size: Int,
    tint: Int,
    alpha: Float = 1f,
    rotationDegrees: Float = 0f,
) {
    drawable ?: return
    drawable.setTint(tint)
    drawable.alpha = (alpha.coerceIn(0f, 1f) * 255).toInt()
    val half = size / 2
    drawable.setBounds(
        centerX.toInt() - half,
        centerY.toInt() - half,
        centerX.toInt() + half,
        centerY.toInt() + half,
    )
    val saveCount = canvas.save()
    canvas.rotate(rotationDegrees, centerX, centerY)
    drawable.draw(canvas)
    canvas.restoreToCount(saveCount)
    drawable.alpha = 255
}

internal inline fun withPaintAlpha(paint: Paint, alpha: Int, block: () -> Unit) {
    val previous = paint.alpha
    paint.alpha = alpha.coerceIn(0, 255)
    block()
    paint.alpha = previous
}

/** Three dots with a travelling wave; [chasing] strengthens the wave. */
internal fun drawActivityDots(
    canvas: Canvas,
    paint: Paint,
    centerX: Float,
    centerY: Float,
    reveal: Float,
    chasing: Boolean,
    tint: Int,
    phase: Float,
    spacing: Float,
    baseRadius: Float,
) {
    val alphaProgress = ((reveal - 0.28f) / 0.52f).coerceIn(0f, 1f)
    val start = centerX - spacing
    repeat(3) { index ->
        val wave = (sin(phase * 2f * PI.toFloat() - index * 0.9f) + 1f) / 2f
        paint.color = tint
        paint.alpha = (
            alphaProgress * if (chasing) 90 + wave * 165 else 145 + wave * 90
        ).toInt().coerceIn(0, 255)
        canvas.drawCircle(start + spacing * index, centerY, baseRadius * (0.82f + wave * 0.32f), paint)
    }
    paint.alpha = 255
}
