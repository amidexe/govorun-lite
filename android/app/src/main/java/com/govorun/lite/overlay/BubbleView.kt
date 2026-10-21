package com.govorun.lite.overlay

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.util.AttributeSet
import android.view.View
import android.view.animation.PathInterpolator
import androidx.core.content.ContextCompat
import com.govorun.lite.R
import com.govorun.lite.util.Prefs
import kotlin.math.max

/**
 * Floating control whose primary tap target never moves.
 *
 * The idle, recording and processing bird share one anchored
 * centre. Contextual controls grow only towards the free side of the screen,
 * so the user can start and finish dictation without aiming twice.
 */
class BubbleView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0,
) : View(context, attrs, defStyleAttr) {

    enum class Presentation {
        IDLE,
        LEGACY_RECORDING,
        HOLD_RECORDING,
        CONFIRM_RECORDING,
        PROCESSING,
        FALLBACK,
    }

    enum class Action {
        NONE,
        CANCEL,
        CONFIRM,
    }

    private val dp = resources.displayMetrics.density
    private var sizeScale = Prefs.getBubbleSize(context)
    private var bubbleSize = scaledPx(BASE_BUBBLE_DP)
    private var iconSize = scaledPx(BASE_ICON_DP)
    private var idleExtent = bubbleTouchExtentPx(dp, sizeScale)
    private var windowExtent = scaledPx(MAX_CONTROL_WIDTH_DP).coerceAtLeast(idleExtent)
    private var onboardingPresentation = false

    private val palette = BubblePalette(context, ::invalidate)
    private val animations = BubbleAnimations(::invalidate)
    private var presentation = Presentation.IDLE
    private var previousPresentation = Presentation.IDLE
    private var anchoredLeft = Prefs.getBubbleSide(context) == Prefs.BUBBLE_SIDE_LEFT
    private var animatedWidth = idleExtent.toFloat()
    private var transitionProgress = 1f
    private var transitionAnimator: ValueAnimator? = null
    private val fallbackSignal = BubbleFallbackSignal(
        invalidate = ::invalidate,
        onFinished = { setPresentation(Presentation.IDLE, anchoredLeft) },
    )

    private val birdIcon =
        ContextCompat.getDrawable(context, R.drawable.ic_bird_24)?.mutate()
    private val cancelIcon =
        ContextCompat.getDrawable(context, R.drawable.ic_close_24)?.mutate()

    init {
        elevation = 6 * dp
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_YES
        palette.setBirdColor(Prefs.getBirdColor(context))
        updateContentDescription()
    }

    fun setPresentation(next: Presentation, anchorOnLeft: Boolean = anchoredLeft) {
        anchoredLeft = anchorOnLeft
        // Keep the brief fallback visible after the recognition callback clears processing.
        if (next == Presentation.IDLE && fallbackSignal.isVisible()) return
        if (presentation == next && animatedWidth.toInt() == targetWidth(next)) {
            invalidate()
            return
        }

        val outgoingPresentation = presentation
        previousPresentation = outgoingPresentation
        presentation = next
        updateContentDescription()
        animations.sync(presentation, bubbleSize)

        val fromWidth = animatedWidth
        val toWidth = targetWidth(next).toFloat()
        transitionAnimator?.cancel()
        if (fromWidth == toWidth) {
            animatedWidth = toWidth
            transitionProgress = 1f
            requestLayout()
            invalidate()
            return
        }

        transitionProgress = 0f
        val collapsingToIdle = outgoingPresentation.isContextual() &&
            !next.isContextual()
        // Resize the stable overlay once so its anchored bird never drifts.
        if (!collapsingToIdle) {
            animatedWidth = toWidth
            requestLayout()
        }
        transitionAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = TRANSITION_DURATION_MS
            interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
            var cancelled = false
            addUpdateListener {
                val progress = it.animatedValue as Float
                transitionProgress = progress
                invalidate()
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: Animator) {
                    if (!cancelled && collapsingToIdle) {
                        animatedWidth = toWidth
                        transitionProgress = 1f
                        requestLayout()
                        invalidate()
                    }
                }
            })
            start()
        }
    }

    fun setAnchoredLeft(anchorOnLeft: Boolean) {
        anchoredLeft = anchorOnLeft
        invalidate()
    }

    fun isConfirmationRecording(): Boolean =
        presentation == Presentation.CONFIRM_RECORDING

    fun isProcessing(): Boolean = presentation == Presentation.PROCESSING

    /**
     * The whole touch window is active, not just the drawn rail, so misses
     * around the pill still count. Cancel keeps the minimum touch target at the
     * far end; every other point confirms (see [isCancelTarget]).
     */
    fun actionAt(x: Float, y: Float): Action {
        if (!isConfirmationRecording()) return Action.NONE
        if (y < 0f || y > height) return Action.NONE
        val geometry = contextualGeometry()
        val cancels = isCancelTarget(
            x = x,
            cancelX = geometry.cancelX,
            rail = geometry.rail,
            anchoredLeft = anchoredLeft,
            minTargetPx = MIN_TOUCH_TARGET_DP * dp,
        )
        return if (cancels) Action.CANCEL else Action.CONFIRM
    }

    fun actionAtControl(x: Float, y: Float): Action {
        val offsetX = if (anchoredLeft) 0f else (width - controlWidthPx()).toFloat()
        return actionAt(x + offsetX, y)
    }

    fun controlWidthPx(): Int = animatedWidth.toInt().coerceAtLeast(idleExtent)

    fun inputWidthPx(): Int = inputBubbleWidth(presentation, controlWidthPx(), idleExtent)

    fun controlHeightPx(): Int = idleExtent

    fun currentPresentation(): Presentation = presentation

    fun setRecording(recording: Boolean) {
        setPresentation(if (recording) Presentation.LEGACY_RECORDING else Presentation.IDLE)
    }

    fun setProcessing(processing: Boolean) {
        setPresentation(if (processing) Presentation.PROCESSING else Presentation.IDLE)
    }

    fun setIdlePulseColor(color: Int) = palette.setIdlePulseColor(color)

    fun setIdleAlpha(alpha: Float) = palette.setIdleAlpha(alpha)

    fun setRecordingAlpha(alpha: Float) = palette.setRecordingAlpha(alpha)

    fun setBirdColor(id: String) = palette.setBirdColor(id)

    /** Ignores size and side preferences for the centred onboarding action; keeps the colour. */
    fun configureForOnboarding(compactModePreview: Boolean = false) {
        if (onboardingPresentation) return
        onboardingPresentation = true
        sizeScale = if (compactModePreview) 0.75f else 1f
        bubbleSize = scaledPx(BASE_BUBBLE_DP)
        iconSize = scaledPx(BASE_ICON_DP)
        idleExtent = if (compactModePreview) bubbleTouchExtentPx(dp, sizeScale)
            else scaledPx(ONBOARDING_EXTENT_DP)
        windowExtent = if (compactModePreview) scaledPx(MAX_CONTROL_WIDTH_DP) else idleExtent
        animatedWidth = idleExtent.toFloat()
        anchoredLeft = true
        palette.setIdleAlpha(Prefs.getBubbleAlpha(context))
        updateContentDescription()
        animations.sync(presentation, bubbleSize)
        requestLayout()
        invalidate()
    }

    fun applySizeScaleFromPrefs(): Boolean {
        val newScale = Prefs.getBubbleSize(context)
        if (!onboardingPresentation && newScale == sizeScale) return false
        onboardingPresentation = false
        sizeScale = newScale
        bubbleSize = scaledPx(BASE_BUBBLE_DP)
        iconSize = scaledPx(BASE_ICON_DP)
        idleExtent = bubbleTouchExtentPx(dp, sizeScale)
        windowExtent = scaledPx(MAX_CONTROL_WIDTH_DP).coerceAtLeast(idleExtent)
        animatedWidth = targetWidth(presentation).toFloat()
        requestLayout()
        invalidate()
        return true
    }

    fun refreshThemeColors() = palette.refresh()

    fun setThemeColors(colors: BubbleThemeColors) = palette.setThemeColors(colors)

    fun setIdlePulse(pulse: Boolean) {
        animations.setIdlePulse(pulse, presentation, bubbleSize)
    }

    fun showStylizationFallback() {
        setPresentation(Presentation.FALLBACK, anchoredLeft)
        fallbackSignal.show()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        setMeasuredDimension(windowExtent.coerceAtLeast(1), idleExtent)
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val saveCount = canvas.save()
        canvas.translate(fallbackSignal.offsetX(scaledDp(3f)), 0f)
        when (presentation) {
            Presentation.IDLE -> drawIdle(canvas)
            Presentation.LEGACY_RECORDING -> drawLegacyRecording(canvas)
            Presentation.HOLD_RECORDING -> drawContextual(canvas, ContextualContent.HOLD)
            Presentation.CONFIRM_RECORDING -> drawContextual(canvas, ContextualContent.CONFIRM)
            Presentation.PROCESSING -> drawContextual(canvas, ContextualContent.PROCESSING)
            Presentation.FALLBACK -> drawContextual(canvas, ContextualContent.FALLBACK)
        }
        canvas.restoreToCount(saveCount)
    }

    private fun drawIdle(canvas: Canvas) {
        val collapsing = previousPresentation.isContextual() && transitionProgress < 1f
        if (collapsing) {
            val reveal = 1f - transitionProgress
            drawRail(canvas, contextualGeometry(reveal))
        }
        drawRoundBubble(
            canvas,
            recording = false,
            drawBackground = !collapsing,
            birdColor = if (collapsing) palette.railForeground(recordingShare()) else null,
        )
    }

    private fun drawLegacyRecording(canvas: Canvas) {
        drawRoundBubble(canvas, recording = true)
    }

    private fun drawRoundBubble(
        canvas: Canvas,
        recording: Boolean,
        drawBackground: Boolean = true,
        birdColor: Int? = null,
    ) {
        val cx = anchorCenterX()
        val cy = height / 2f
        val idleAnimating = animations.idleHaloActive && !recording
        val radius = bubbleSize / 2f

        if (idleAnimating && animations.idleHaloAlpha > 0f) {
            withPaintAlpha(
                palette.idleHaloPaint,
                (animations.idleHaloAlpha * 255f).toInt(),
            ) {
                canvas.drawCircle(cx, cy, animations.idleHaloRadius, palette.idleHaloPaint)
            }
        }
        if (recording && animations.recordingHaloRadius > 0f) {
            canvas.drawCircle(
                cx,
                cy,
                animations.recordingHaloRadius,
                palette.legacyRecordingHaloPaint,
            )
        }
        if (drawBackground) {
            canvas.drawCircle(
                cx,
                cy,
                radius,
                if (recording) palette.legacyRecordingPaint else palette.basePaint,
            )
        }
        drawDrawable(
            canvas,
            birdIcon,
            cx,
            cy,
            iconSize,
            if (recording) palette.recordingForeground() else birdColor ?: palette.idleBirdColor(),
            rotationDegrees = 0f,
        )
    }

    private fun drawContextual(canvas: Canvas, content: ContextualContent) {
        val reveal = contextualReveal()
        val geometry = contextualGeometry(reveal)
        val share = recordingShare()
        val foreground = palette.railForeground(share)
        drawRail(canvas, geometry)

        when (content) {
            ContextualContent.CONFIRM -> drawConfirmContent(canvas, geometry, reveal)
            ContextualContent.HOLD -> {
                drawDrawable(
                    canvas,
                    birdIcon,
                    geometry.anchorX,
                    geometry.centerY,
                    iconSize,
                    foreground,
                    rotationDegrees = 0f,
                )
                drawDots(canvas, geometry.dotsX, geometry.centerY, reveal, chasing = false, tint = foreground)
            }
            ContextualContent.PROCESSING -> {
                drawDrawable(
                    canvas,
                    birdIcon,
                    geometry.anchorX,
                    geometry.centerY,
                    iconSize,
                    foreground,
                    rotationDegrees = 0f,
                )
                drawDots(
                    canvas,
                    geometry.dotsX,
                    geometry.centerY,
                    reveal = 1f,
                    chasing = true,
                )
            }
            ContextualContent.FALLBACK -> drawDrawable(
                canvas,
                birdIcon,
                geometry.anchorX,
                geometry.centerY,
                iconSize,
                foreground,
            )
        }
    }

    private fun drawConfirmContent(
        canvas: Canvas,
        geometry: ContextualGeometry,
        reveal: Float,
    ) {
        val foreground = palette.railForeground(recordingShare())
        drawDrawable(
            canvas,
            birdIcon,
            geometry.anchorX,
            geometry.centerY,
            iconSize,
            foreground,
            rotationDegrees = 0f,
        )
        drawDots(canvas, geometry.dotsX, geometry.centerY, reveal, chasing = false, tint = foreground)

        val cancelProgress = ((reveal - 0.48f) / 0.52f).coerceIn(0f, 1f)
        drawDrawable(
            canvas,
            cancelIcon,
            geometry.cancelX,
            geometry.centerY,
            scaledPx(20f),
            foreground,
            cancelProgress,
        )
    }

    private fun drawRail(
        canvas: Canvas,
        geometry: ContextualGeometry = contextualGeometry(),
    ) {
        palette.railPaint.color = palette.railFill(recordingShare())
        canvas.drawRoundRect(
            geometry.rail,
            geometry.rail.height() / 2f,
            geometry.rail.height() / 2f,
            palette.railPaint,
        )
    }

    private fun drawDots(
        canvas: Canvas,
        centerX: Float,
        centerY: Float,
        reveal: Float,
        chasing: Boolean,
        tint: Int = palette.idleBirdColor(),
    ) = drawActivityDots(
        canvas, palette.dotPaint, centerX, centerY, reveal, chasing, tint,
        animations.activityPhase, scaledDp(7f), max(1.45f * dp, scaledDp(1.8f)),
    )

    private fun contextualGeometry(reveal: Float = contextualReveal()): ContextualGeometry {
        return buildAnchoredContextualGeometry(
            windowWidth = width,
            controlWidth = animatedWidth.toInt(),
            height = height,
            idleExtent = idleExtent,
            bubbleSize = bubbleSize,
            anchoredLeft = anchoredLeft,
            // The circle only stretches sideways into the rail, never changes height.
            finalHeight = bubbleSize.toFloat(),
            reveal = reveal,
            innerPadding = scaledDp(4f),
        )
    }

    /** 1 while the rail records (tap or hold), 0 otherwise; eased by the same transition. */
    private fun recordingShare(): Float {
        fun share(value: Presentation) =
            if (value == Presentation.CONFIRM_RECORDING || value == Presentation.HOLD_RECORDING) 1f else 0f
        val from = share(previousPresentation)
        return from + (share(presentation) - from) * transitionProgress
    }

    private fun contextualReveal(): Float =
        if (previousPresentation.isContextual()) 1f else transitionProgress

    private fun anchorCenterX(): Float =
        if (anchoredLeft) idleExtent / 2f else width - idleExtent / 2f

    private fun targetWidth(value: Presentation): Int = targetBubbleWidth(
        value = value,
        previous = previousPresentation,
        animatedWidth = animatedWidth,
        idleExtent = idleExtent,
        densityScale = dp * sizeScale,
    )

    private fun updateContentDescription() {
        contentDescription = context.getString(
            bubbleContentDescriptionRes(presentation, onboardingPresentation),
        )
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        transitionAnimator?.cancel()
        fallbackSignal.cancel()
        animations.cancelAll()
    }

    private fun scaledDp(value: Float) = value * dp * sizeScale

    private fun scaledPx(value: Float) = scaledDp(value).toInt()

    private companion object {
        const val BASE_BUBBLE_DP = BUBBLE_BASE_DP
        const val BASE_ICON_DP = 32f
        const val MAX_CONTROL_WIDTH_DP = 152f
        const val ONBOARDING_EXTENT_DP = 96f
        const val TRANSITION_DURATION_MS = 280L
    }
}
