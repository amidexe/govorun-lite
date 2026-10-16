package com.govorun.lite.overlay

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
/** Owns the bounded whole-surface failure signal after online styling fallback. */
internal class BubbleFallbackSignal(
    private val invalidate: () -> Unit,
    private val onFinished: () -> Unit,
) {
    private var shakeProgress = 0f
    private var visible = false
    private var animator: ValueAnimator? = null

    fun show() {
        animator?.cancel()
        shakeProgress = 0f
        visible = true
        animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = SHAKE_DURATION_MS
            addUpdateListener {
                shakeProgress = it.animatedValue as Float
                invalidate()
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    shakeProgress = 0f
                    visible = false
                    invalidate()
                    onFinished()
                }
            })
            start()
        }
    }

    fun offsetX(amplitudePx: Float): Float = if (visible) fallbackShakeOffset(
        shakeProgress,
        amplitudePx,
    ) else 0f

    fun isVisible(): Boolean = visible

    fun cancel() = animator?.cancel()

    private companion object {
        const val SHAKE_DURATION_MS = 360L
    }
}
