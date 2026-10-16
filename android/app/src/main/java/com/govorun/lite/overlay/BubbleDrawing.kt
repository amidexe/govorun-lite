package com.govorun.lite.overlay

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.drawable.Drawable

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

internal fun insideAction(
    x: Float,
    y: Float,
    centerX: Float,
    centerY: Float,
    radius: Float,
): Boolean {
    val dx = x - centerX
    val dy = y - centerY
    return dx * dx + dy * dy <= radius * radius
}
