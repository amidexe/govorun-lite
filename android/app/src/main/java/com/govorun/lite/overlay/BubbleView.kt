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
import kotlin.math.PI
import kotlin.math.max
import kotlin.math.sin

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
    private var idleExtent = bubbleSize + (bubbleSize * IDLE_PADDING_FRACTION).toInt()
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

    fun actionAt(x: Float, y: Float): Action {
        if (!isConfirmationRecording()) return Action.NONE
        val geometry = contextualGeometry()
        if (insideAction(x, y, geometry.anchorX, geometry.centerY, geometry.actionRadius)) {
            return Action.CONFIRM
        }
        return if (insideAction(
                x,
                y,
                geometry.cancelX,
                geometry.centerY,
                geometry.actionRadius * 0.82f,
            )
        ) {
            Action.CANCEL
        } else {
            Action.NONE
        }
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

    /** Ignores floating-overlay preferences for the centred onboarding action. */
    fun configureForOnboarding(compactModePreview: Boolean = false) {
        if (onboardingPresentation) return
        onboardingPresentation = true
        sizeScale = if (compactModePreview) 0.75f else 1f
        bubbleSize = scaledPx(BASE_BUBBLE_DP)
        iconSize = scaledPx(BASE_ICON_DP)
        idleExtent = if (compactModePreview) bubbleSize + (bubbleSize * IDLE_PADDING_FRACTION).toInt()
            else scaledPx(ONBOARDING_EXTENT_DP)
        windowExtent = if (compactModePreview) scaledPx(MAX_CONTROL_WIDTH_DP) else idleExtent
        animatedWidth = idleExtent.toFloat()
        anchoredLeft = true
        palette.setIdleAlpha(1f)
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
        idleExtent = bubbleSize + (bubbleSize * IDLE_PADDING_FRACTION).toInt()
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
        drawRoundBubble(canvas, recording = false, drawBackground = !collapsing)
    }

    private fun drawLegacyRecording(canvas: Canvas) {
        drawRoundBubble(canvas, recording = true)
    }

    private fun drawRoundBubble(canvas: Canvas, recording: Boolean, drawBackground: Boolean = true) {
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
            if (recording) 0xFFFFFFFF.toInt() else palette.idleBirdColor(),
            rotationDegrees = 0f,
        )
    }

    private fun drawContextual(canvas: Canvas, content: ContextualContent) {
        val reveal = contextualReveal()
        val geometry = contextualGeometry(reveal)
        val foreground = palette.idleBirdColor()
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
                drawDots(canvas, geometry.dotsX, geometry.centerY, reveal, chasing = false)
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
        val foreground = palette.idleBirdColor()
        drawDrawable(
            canvas,
            birdIcon,
            geometry.anchorX,
            geometry.centerY,
            iconSize,
            foreground,
            rotationDegrees = 0f,
        )
        drawDots(canvas, geometry.dotsX, geometry.centerY, reveal, chasing = false)

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
        canvas.drawRoundRect(
            geometry.rail,
            geometry.rail.height() / 2f,
            geometry.rail.height() / 2f,
            palette.basePaint,
        )
    }

    private fun drawDots(
        canvas: Canvas,
        centerX: Float,
        centerY: Float,
        reveal: Float,
        chasing: Boolean,
    ) {
        val alphaProgress = ((reveal - 0.28f) / 0.52f).coerceIn(0f, 1f)
        val spacing = scaledDp(7f)
        val start = centerX - spacing
        repeat(3) { index ->
            val wave = (
                sin(animations.activityPhase * 2f * PI.toFloat() - index * 0.9f) + 1f
            ) / 2f
            val radius = max(1.45f * dp, scaledDp(1.8f)) * (0.82f + wave * 0.32f)
            palette.dotPaint.color = palette.idleBirdColor()
            palette.dotPaint.alpha = (
                alphaProgress * if (chasing) 90 + wave * 165 else 145 + wave * 90
            ).toInt().coerceIn(0, 255)
            canvas.drawCircle(start + spacing * index, centerY, radius, palette.dotPaint)
        }
        palette.dotPaint.alpha = 255
    }

    private fun contextualGeometry(reveal: Float = contextualReveal()): ContextualGeometry {
        return buildAnchoredContextualGeometry(
            windowWidth = width,
            controlWidth = animatedWidth.toInt(),
            height = height,
            idleExtent = idleExtent,
            bubbleSize = bubbleSize,
            anchoredLeft = anchoredLeft,
            finalHeight = scaledDp(48f),
            reveal = reveal,
            innerPadding = scaledDp(4f),
        )
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
        const val BASE_BUBBLE_DP = 56f
        const val BASE_ICON_DP = 32f
        const val MAX_CONTROL_WIDTH_DP = 152f
        const val ONBOARDING_EXTENT_DP = 96f
        const val IDLE_PADDING_FRACTION = 0.4f
        const val TRANSITION_DURATION_MS = 280L
    }
}
