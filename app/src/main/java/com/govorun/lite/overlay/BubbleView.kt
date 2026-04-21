package com.govorun.lite.overlay

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.drawable.Drawable
import android.util.AttributeSet
import android.util.TypedValue
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import androidx.core.content.ContextCompat
import com.govorun.lite.R

/**
 * The floating "Говорун" — a round button with the bird mascot. Idle state
 * uses M3 colorPrimaryContainer (picks up Dynamic Colors from the wallpaper);
 * recording state is always red — the universal REC signal shouldn't be
 * tinted to the user's accent, that would blur the "you are being recorded"
 * affordance.
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

    // Fallbacks if the host theme lacks M3 attrs (shouldn't happen — the
    // app theme is Theme.GovorunLite, Material3 — but keep the bubble
    // visible regardless).
    private val fallbackIdleFill = 0x80404040.toInt()
    private val fallbackIdleTint = 0xFFFFFFFF.toInt()
    private val recordingFill = 0xFFE53935.toInt()
    private val processingFill = 0xFFFFA726.toInt()
    private val recordingBirdTint = 0xFFFFFFFF.toInt()

    // Theme-derived RGB (without alpha) — so that changing the user-chosen
    // transparency doesn't require re-reading the theme, and theme refresh
    // (wallpaper colour change) doesn't clobber the transparency setting.
    private var idleFillRgb = readThemeRgb(
        com.google.android.material.R.attr.colorPrimaryContainer,
        fallbackIdleFill
    )
    private var idleBirdTint = readThemeRgb(
        com.google.android.material.R.attr.colorOnPrimaryContainer,
        fallbackIdleTint
    ) or 0xFF000000.toInt()
    private var haloBaseColor = readThemeRgb(
        com.google.android.material.R.attr.colorPrimaryContainer,
        recordingFill
    ) or 0xFF000000.toInt()

    // User-configurable fill alpha (from Prefs; see Prefs.BUBBLE_ALPHA_*).
    // Held separately so refreshThemeColors() never resets it. Kept in sync
    // with Prefs.BUBBLE_ALPHA_DEFAULT — if you change one, change both.
    private var idleAlphaFraction: Float = 0.85f

    private val basePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = composeIdleFill()
        style = Paint.Style.FILL
    }
    private val recordingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = recordingFill; style = Paint.Style.FILL
    }
    private val processingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = processingFill; style = Paint.Style.FILL
    }
    private val pulsePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0x40E53935.toInt(); style = Paint.Style.FILL
    }
    // Soft pulsing halo behind the idle bubble — an expanding filled disc
    // whose alpha fades to zero as it grows. Replaces the old swept stroke
    // ring (which read as a loading spinner, not a "tap me" cue).
    private val idleHaloPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = haloBaseColor; style = Paint.Style.FILL
    }

    private var isRecording = false
    private var isProcessing = false
    private var pulseRadius = 0f
    private var pulseAnimator: ValueAnimator? = null

    private var idleHaloActive = false
    private var idleHaloRadius = 0f
    private var idleHaloAlpha = 0f
    private var idleBubbleScale = 1f
    private var idleHaloAnimator: ValueAnimator? = null

    private val birdIcon: Drawable? = ContextCompat.getDrawable(context, R.drawable.ic_bird_24)

    init { elevation = 6 * dp }

    fun setRecording(recording: Boolean) {
        isRecording = recording; isProcessing = false
        if (recording) {
            stopIdleHalo()
            startRecordingPulse()
        } else {
            stopRecordingPulse()
            if (idleHaloActive) startIdleHalo()
        }
        invalidate()
    }

    fun setProcessing(processing: Boolean) {
        isProcessing = processing; isRecording = false
        stopRecordingPulse()
        if (processing) stopIdleHalo()
        else if (idleHaloActive) startIdleHalo()
        invalidate()
    }

    /**
     * Tint the halo pulse. The fragment may pass a specific M3 attr color;
     * otherwise the constructor picks colorPrimaryContainer from the theme.
     */
    fun setIdlePulseColor(color: Int) {
        haloBaseColor = (color and 0x00FFFFFF) or 0xFF000000.toInt()
        idleHaloPaint.color = haloBaseColor
        invalidate()
    }

    /**
     * Apply a user-chosen transparency to the idle bubble fill. Value is
     * clamped to [0, 1] here; callers (Prefs) are expected to have already
     * clamped to the project's min/max range.
     */
    fun setIdleAlpha(alpha: Float) {
        idleAlphaFraction = alpha.coerceIn(0f, 1f)
        basePaint.color = composeIdleFill()
        invalidate()
    }

    /**
     * Re-read colorPrimaryContainer / colorOnPrimaryContainer from the current
     * theme. Call this after Dynamic Colors might have changed (e.g. the user
     * switched wallpaper) to pick up the new system accent without destroying
     * the bubble view.
     */
    fun refreshThemeColors() {
        idleFillRgb = readThemeRgb(
            com.google.android.material.R.attr.colorPrimaryContainer,
            fallbackIdleFill
        )
        idleBirdTint = readThemeRgb(
            com.google.android.material.R.attr.colorOnPrimaryContainer,
            fallbackIdleTint
        ) or 0xFF000000.toInt()
        haloBaseColor = readThemeRgb(
            com.google.android.material.R.attr.colorPrimaryContainer,
            recordingFill
        ) or 0xFF000000.toInt()
        basePaint.color = composeIdleFill()
        idleHaloPaint.color = haloBaseColor
        invalidate()
    }

    /**
     * Toggle the feature-highlight halo. Used on the onboarding demo step
     * so the "tap me" target is obvious. Automatically pauses while the
     * bubble is recording or processing.
     */
    fun setIdlePulse(pulse: Boolean) {
        if (idleHaloActive == pulse) return
        idleHaloActive = pulse
        if (pulse && !isRecording && !isProcessing) startIdleHalo()
        else if (!pulse) stopIdleHalo()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val size = bubbleSize + (bubbleSize * 0.6f).toInt()
        setMeasuredDimension(size, size)
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val cx = width / 2f; val cy = height / 2f
        val inIdleAnim = idleHaloActive && !isRecording && !isProcessing
        val scale = if (inIdleAnim) idleBubbleScale else 1f
        val radius = bubbleSize / 2f * scale
        val scaledIcon = (iconSize * scale).toInt()

        // 1. Idle halo underneath the bubble. Expands outward from the
        //    scaled bubble edge and fades to zero so it reads as a
        //    breathing glow, not a static ring.
        if (inIdleAnim && idleHaloAlpha > 0f) {
            val saved = idleHaloPaint.alpha
            idleHaloPaint.alpha = (idleHaloAlpha * 255f).toInt().coerceIn(0, 255)
            canvas.drawCircle(cx, cy, idleHaloRadius, idleHaloPaint)
            idleHaloPaint.alpha = saved
        }

        // 2. Recording pulse.
        if (pulseRadius > 0 && isRecording) {
            canvas.drawCircle(cx, cy, pulseRadius, pulsePaint)
        }

        // 3. Main bubble disc.
        val paint = when {
            isProcessing -> processingPaint
            isRecording -> recordingPaint
            else -> basePaint
        }
        canvas.drawCircle(cx, cy, radius, paint)

        // 4. Bird on top. Tint swaps with state so contrast holds on any
        //    Dynamic-Colors palette. Icon scales with the bubble so the
        //    proportions stay right during the breathing animation.
        birdIcon?.let {
            val birdTint = if (isRecording || isProcessing) recordingBirdTint else idleBirdTint
            it.setTint(birdTint)
            val l = (cx - scaledIcon / 2).toInt()
            val t = (cy - scaledIcon / 2).toInt()
            it.setBounds(l, t, l + scaledIcon, t + scaledIcon)
            it.draw(canvas)
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

    // Combined breathing animation: the bubble itself scales 1.0 → 1.07 → 1.0
    // while a halo expands outward from its edge and fades to zero. The two
    // motions share one animator so they stay in phase — reads as a single
    // gentle breath instead of two disconnected effects.
    private fun startIdleHalo() {
        idleHaloAnimator?.cancel()
        val innerR = bubbleSize / 2f
        val outerR = bubbleSize / 2f * 1.7f
        idleHaloAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1500L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener {
                val t = it.animatedValue as Float
                idleHaloRadius = innerR + (outerR - innerR) * t
                idleHaloAlpha = 0.55f * (1f - t)
                // sin(πt) — smooth zero-peak-zero curve over the cycle, so
                // the bubble grows as the halo leaves and settles back just
                // before the next halo starts.
                idleBubbleScale = 1f + 0.07f * kotlin.math.sin(t * Math.PI).toFloat()
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
        idleBubbleScale = 1f
        invalidate()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        pulseAnimator?.cancel()
        idleHaloAnimator?.cancel()
    }

    // Theme lookup wrapper: keeps the whole bubble working even if someone
    // ever constructs it from a non-Material context. Returns only the RGB
    // channels (alpha bits always 0) — caller composes the final colour with
    // the separately-stored user alpha so the two concerns don't mix.
    private fun readThemeRgb(attr: Int, fallback: Int): Int {
        val tv = TypedValue()
        val resolved = context.theme.resolveAttribute(attr, tv, true)
        val raw = if (resolved) {
            if (tv.resourceId != 0) ContextCompat.getColor(context, tv.resourceId) else tv.data
        } else {
            fallback
        }
        return raw and 0x00FFFFFF
    }

    private fun composeIdleFill(): Int {
        val a = (idleAlphaFraction.coerceIn(0f, 1f) * 255).toInt()
        return (a shl 24) or (idleFillRgb and 0x00FFFFFF)
    }
}
