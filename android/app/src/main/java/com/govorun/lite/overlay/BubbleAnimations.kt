package com.govorun.lite.overlay

import android.animation.ValueAnimator
import android.view.animation.AccelerateDecelerateInterpolator
import android.view.animation.LinearInterpolator

internal class BubbleAnimations(
    private val invalidate: () -> Unit,
) {
    var activityPhase = 0f
        private set
    var recordingHaloRadius = 0f
        private set
    var idleHaloRadius = 0f
        private set
    var idleHaloAlpha = 0f
        private set
    var idleHaloActive = false
        private set

    private var activityAnimator: ValueAnimator? = null
    private var recordingHaloAnimator: ValueAnimator? = null
    private var idleHaloAnimator: ValueAnimator? = null

    fun sync(presentation: BubbleView.Presentation, bubbleSize: Int) {
        if (presentation == BubbleView.Presentation.LEGACY_RECORDING) {
            startRecordingHalo(bubbleSize)
        } else {
            stopRecordingHalo()
        }
        val usesDots = presentation == BubbleView.Presentation.HOLD_RECORDING ||
            presentation == BubbleView.Presentation.CONFIRM_RECORDING ||
            presentation == BubbleView.Presentation.PROCESSING
        if (usesDots) startActivity() else stopActivity()
        if (presentation == BubbleView.Presentation.IDLE && idleHaloActive) {
            startIdleHalo(bubbleSize)
        } else {
            stopIdleHalo()
        }
    }

    fun setIdlePulse(active: Boolean, presentation: BubbleView.Presentation, bubbleSize: Int) {
        if (idleHaloActive == active) return
        idleHaloActive = active
        if (active && presentation == BubbleView.Presentation.IDLE) startIdleHalo(bubbleSize)
        else if (!active) stopIdleHalo()
    }

    fun cancelAll() {
        activityAnimator?.cancel()
        recordingHaloAnimator?.cancel()
        idleHaloAnimator?.cancel()
    }

    private fun startActivity() {
        if (activityAnimator?.isRunning == true) return
        activityAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 900L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener {
                activityPhase = it.animatedValue as Float
                invalidate()
            }
            start()
        }
    }

    private fun stopActivity() {
        activityAnimator?.cancel()
        activityAnimator = null
        activityPhase = 0f
    }

    private fun startRecordingHalo(bubbleSize: Int) {
        if (recordingHaloAnimator?.isRunning == true) return
        recordingHaloAnimator = ValueAnimator.ofFloat(
            bubbleSize / 2f,
            bubbleSize / 2f * 1.4f,
        ).apply {
            duration = 800L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.REVERSE
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener {
                recordingHaloRadius = it.animatedValue as Float
                invalidate()
            }
            start()
        }
    }

    private fun stopRecordingHalo() {
        recordingHaloAnimator?.cancel()
        recordingHaloAnimator = null
        recordingHaloRadius = 0f
    }

    private fun startIdleHalo(bubbleSize: Int) {
        idleHaloAnimator?.cancel()
        val innerRadius = bubbleSize / 2f
        val outerRadius = bubbleSize / 2f * 1.7f
        idleHaloAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1500L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener {
                val progress = it.animatedValue as Float
                idleHaloRadius = innerRadius + (outerRadius - innerRadius) * progress
                idleHaloAlpha = 0.55f * (1f - progress)
                invalidate()
            }
            start()
        }
    }

    private fun stopIdleHalo() {
        idleHaloAnimator?.cancel()
        idleHaloAnimator = null
        idleHaloRadius = 0f
        idleHaloAlpha = 0f
        invalidate()
    }
}
