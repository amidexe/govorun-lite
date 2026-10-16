package com.govorun.lite.overlay

import androidx.annotation.StringRes
import com.govorun.lite.R

internal enum class ContextualContent {
    HOLD,
    CONFIRM,
    PROCESSING,
    FALLBACK,
}

internal fun BubbleView.Presentation.isContextual(): Boolean =
    this == BubbleView.Presentation.HOLD_RECORDING ||
        this == BubbleView.Presentation.CONFIRM_RECORDING ||
        this == BubbleView.Presentation.PROCESSING ||
        this == BubbleView.Presentation.FALLBACK

internal fun targetBubbleWidth(
    value: BubbleView.Presentation,
    previous: BubbleView.Presentation,
    animatedWidth: Float,
    idleExtent: Int,
    densityScale: Float,
): Int = when (value) {
    BubbleView.Presentation.IDLE,
    BubbleView.Presentation.LEGACY_RECORDING,
    -> idleExtent
    BubbleView.Presentation.HOLD_RECORDING -> (124f * densityScale).toInt()
    BubbleView.Presentation.CONFIRM_RECORDING -> (152f * densityScale).toInt()
    BubbleView.Presentation.PROCESSING -> (
        processingControlWidthDp(previous == BubbleView.Presentation.HOLD_RECORDING) *
            densityScale
        ).toInt()
    BubbleView.Presentation.FALLBACK -> animatedWidth.toInt()
}.coerceAtLeast(idleExtent)

internal fun inputBubbleWidth(
    presentation: BubbleView.Presentation,
    animatedWidth: Int,
    idleExtent: Int,
): Int = when (presentation) {
    BubbleView.Presentation.IDLE,
    BubbleView.Presentation.LEGACY_RECORDING,
    -> idleExtent
    else -> animatedWidth.coerceAtLeast(idleExtent)
}

@StringRes
internal fun bubbleContentDescriptionRes(
    presentation: BubbleView.Presentation,
    onboarding: Boolean,
): Int = when (presentation) {
    BubbleView.Presentation.IDLE -> if (onboarding) {
        R.string.onb_try_tap_to_start
    } else {
        R.string.bubble_a11y_idle
    }
    BubbleView.Presentation.LEGACY_RECORDING -> R.string.bubble_a11y_recording
    BubbleView.Presentation.HOLD_RECORDING -> R.string.bubble_a11y_hold_recording
    BubbleView.Presentation.CONFIRM_RECORDING -> R.string.bubble_a11y_confirm_recording
    BubbleView.Presentation.PROCESSING -> R.string.bubble_a11y_processing
    BubbleView.Presentation.FALLBACK -> R.string.bubble_a11y_fallback
}
