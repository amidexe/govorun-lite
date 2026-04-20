package com.govorun.lite.overlay

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.drawable.Drawable
import android.util.AttributeSet
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import androidx.core.content.ContextCompat
import com.govorun.lite.R

/**
 * The floating "Говорун" — a translucent dark circle with a white bird
 * silhouette. Red with pulsing halo while recording, orange while processing.
 *
 * Lite build has no mode dot and no pill expansion — a single VAD mode
 * is the only interaction, so Говорун carries no extra UI.
 */
class BubbleView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    private val dp = resources.displayMetrics.density
    private val bubbleSize = (56 * dp).toInt()
    // Bird silhouette reads a bit small at 24dp on a 56dp disc — bump it up
    // so the shape is recognisable at a glance.
    private val iconSize = (32 * dp).toInt()

    private val basePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0x80404040.toInt(); style = Paint.Style.FILL
    }
    private val recordingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFE53935.toInt(); style = Paint.Style.FILL
    }
    private val processingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFFFA726.toInt(); style = Paint.Style.FILL
    }
    private val pulsePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0x40E53935.toInt(); style = Paint.Style.FILL
    }
    // Material-style feature highlight: a stroked ring that sweeps in
    // around the bubble, holds briefly, then fades. Repeats every few
    // seconds on the onboarding demo so the tappable target is obvious
    // without the harsh continuous pulse we had before.
    private val idleRingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = 2.5f * dp
        strokeCap = Paint.Cap.ROUND
        color = 0xFFE53935.toInt()
    }

    private var isRecording = false
    private var isProcessing = false
    private var pulseRadius = 0f
    private var pulseAnimator: ValueAnimator? = null

    private var idleRingActive = false
    private var idleRingSweep = 0f
    private var idleRingAlpha = 0f
    private var idleRingAnimator: ValueAnimator? = null

    private val birdIcon: Drawable? = ContextCompat.getDrawable(context, R.drawable.ic_bird_24)?.apply {
        setTint(0xFFFFFFFF.toInt())
    }

    init { elevation = 8 * dp }

    fun setRecording(recording: Boolean) {
        isRecording = recording; isProcessing = false
        if (recording) {
            stopIdleRing()
            startRecordingPulse()
        } else {
            stopRecordingPulse()
            if (idleRingActive) startIdleRing()
        }
        invalidate()
    }

    fun setProcessing(processing: Boolean) {
        isProcessing = processing; isRecording = false
        stopRecordingPulse()
        if (processing) stopIdleRing()
        else if (idleRingActive) startIdleRing()
        invalidate()
    }

    /**
     * Tint the feature-highlight ring. The fragment passes colorPrimary so
     * the sweep harmonises with the screen's M3 palette.
     */
    fun setIdlePulseColor(color: Int) {
        idleRingPaint.color = (color and 0x00FFFFFF) or 0xFF000000.toInt()
        invalidate()
    }

    /**
     * Toggle the feature-highlight ring. Used on the onboarding demo step
     * so the "tap me" target is obvious. Automatically pauses while the
     * bubble is recording or processing.
     */
    fun setIdlePulse(pulse: Boolean) {
        if (idleRingActive == pulse) return
        idleRingActive = pulse
        if (pulse && !isRecording && !isProcessing) startIdleRing()
        else if (!pulse) stopIdleRing()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val size = bubbleSize + (bubbleSize * 0.4f).toInt()
        setMeasuredDimension(size, size)
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val cx = width / 2f; val cy = height / 2f
        val radius = bubbleSize / 2f

        if (pulseRadius > 0 && isRecording) {
            canvas.drawCircle(cx, cy, pulseRadius, pulsePaint)
        }

        val paint = when {
            isProcessing -> processingPaint
            isRecording -> recordingPaint
            else -> basePaint
        }
        canvas.drawCircle(cx, cy, radius, paint)

        birdIcon?.let {
            val l = (cx - iconSize / 2).toInt()
            val t = (cy - iconSize / 2).toInt()
            it.setBounds(l, t, l + iconSize, t + iconSize)
            it.draw(canvas)
        }

        if (idleRingActive && idleRingAlpha > 0f && !isRecording && !isProcessing) {
            val savedAlpha = idleRingPaint.alpha
            idleRingPaint.alpha = (idleRingAlpha * 255f).toInt().coerceIn(0, 255)
            val inset = 6f * dp
            val r = radius + inset
            val rect = RectF(cx - r, cy - r, cx + r, cy + r)
            canvas.drawArc(rect, -90f, 360f * idleRingSweep, false, idleRingPaint)
            idleRingPaint.alpha = savedAlpha
        }
    }

    private fun startRecordingPulse() {
        pulseAnimator?.cancel()
        val maxR = bubbleSize / 2f * 1.4f
        pulseAnimator = ValueAnimator.ofFloat(bubbleSize / 2f, maxR).apply {
            duration = 800L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.REVERSE
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener { pulseRadius = it.animatedValue as Float; invalidate() }
            start()
        }
    }

    private fun stopRecordingPulse() {
        pulseAnimator?.cancel()
        pulseAnimator = null
        pulseRadius = 0f
    }

    // Sweep-in / hold / fade-out / rest. A single animator drives all four
    // phases by mapping its 0..1 progress to sweep angle + alpha — cheaper
    // than an AnimatorSet and easier to cancel cleanly.
    private fun startIdleRing() {
        idleRingAnimator?.cancel()
        idleRingAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 3200L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            addUpdateListener {
                val t = it.animatedValue as Float
                val sweepPhase = (t / 0.28f).coerceIn(0f, 1f)
                // ease-out so the stroke settles at full circle instead of
                // snapping to it.
                idleRingSweep = 1f - (1f - sweepPhase) * (1f - sweepPhase)
                idleRingAlpha = when {
                    t < 0.55f -> 1f
                    t < 0.80f -> 1f - (t - 0.55f) / 0.25f
                    else -> 0f
                }
                invalidate()
            }
            start()
        }
    }

    private fun stopIdleRing() {
        idleRingAnimator?.cancel()
        idleRingAnimator = null
        idleRingSweep = 0f
        idleRingAlpha = 0f
        invalidate()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        pulseAnimator?.cancel()
        idleRingAnimator?.cancel()
    }
}
