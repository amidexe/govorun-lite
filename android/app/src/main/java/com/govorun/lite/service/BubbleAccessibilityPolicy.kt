package com.govorun.lite.service

import androidx.annotation.IdRes
import androidx.annotation.StringRes
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleView
import kotlin.math.roundToInt

internal enum class BubbleAccessibilityNode(val virtualId: Int) {
    PRIMARY(1),
    CANCEL(2),
    STATUS(3),
}

internal enum class BubbleAccessibilityCommand {
    TAP,
    START_HOLD,
    STOP_HOLD,
    CONFIRM,
    CANCEL,
}

internal data class BubbleAccessibilityAction(
    @param:IdRes val id: Int,
    @param:StringRes val label: Int,
    val command: BubbleAccessibilityCommand,
)

internal data class BubbleAccessibilitySpec(
    val node: BubbleAccessibilityNode,
    @param:StringRes val label: Int,
    val click: BubbleAccessibilityCommand? = null,
    val actions: List<BubbleAccessibilityAction> = emptyList(),
)

internal data class BubbleInputState(
    val presentation: BubbleView.Presentation,
    val anchoredLeft: Boolean,
    val width: Int,
    val height: Int,
)

internal data class BubbleNodeBounds(
    val left: Int,
    val top: Int,
    val right: Int,
    val bottom: Int,
) {
    fun contains(x: Float, y: Float): Boolean =
        x >= left && x < right && y >= top && y < bottom
}

internal fun bubbleAccessibilitySpecs(
    presentation: BubbleView.Presentation,
): List<BubbleAccessibilitySpec> = when (presentation) {
    BubbleView.Presentation.IDLE -> listOf(
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.PRIMARY,
            label = R.string.bubble_a11y_action_start_recording,
            click = BubbleAccessibilityCommand.TAP,
            actions = listOf(
                BubbleAccessibilityAction(
                    id = R.id.bubble_a11y_action_quick_input,
                    label = R.string.bubble_a11y_action_quick_input,
                    command = BubbleAccessibilityCommand.START_HOLD,
                ),
            ),
        ),
    )
    BubbleView.Presentation.LEGACY_RECORDING -> listOf(
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.PRIMARY,
            label = R.string.bubble_a11y_action_stop_recording,
            click = BubbleAccessibilityCommand.TAP,
        ),
    )
    BubbleView.Presentation.HOLD_RECORDING -> listOf(
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.PRIMARY,
            label = R.string.bubble_a11y_action_finish_and_insert,
            click = BubbleAccessibilityCommand.STOP_HOLD,
            actions = listOf(
                BubbleAccessibilityAction(
                    id = R.id.bubble_a11y_action_cancel_recording,
                    label = R.string.bubble_a11y_action_cancel_recording,
                    command = BubbleAccessibilityCommand.CANCEL,
                ),
            ),
        ),
    )
    BubbleView.Presentation.CONFIRM_RECORDING -> listOf(
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.PRIMARY,
            label = R.string.bubble_a11y_action_finish_and_insert,
            click = BubbleAccessibilityCommand.CONFIRM,
        ),
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.CANCEL,
            label = R.string.bubble_a11y_action_cancel_recording,
            click = BubbleAccessibilityCommand.CANCEL,
        ),
    )
    BubbleView.Presentation.PROCESSING -> listOf(
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.STATUS,
            label = R.string.bubble_a11y_processing,
        ),
    )
    BubbleView.Presentation.FALLBACK -> listOf(
        BubbleAccessibilitySpec(
            node = BubbleAccessibilityNode.STATUS,
            label = R.string.bubble_a11y_fallback,
        ),
    )
}

internal fun bubbleAccessibilityBounds(
    state: BubbleInputState,
    node: BubbleAccessibilityNode,
): BubbleNodeBounds {
    val width = state.width.coerceAtLeast(1)
    val height = state.height.coerceAtLeast(1)
    if (node == BubbleAccessibilityNode.STATUS) {
        return BubbleNodeBounds(0, 0, width, height)
    }

    val primaryExtent = minOf(width, height)
    if (node == BubbleAccessibilityNode.PRIMARY) {
        return if (state.anchoredLeft) {
            BubbleNodeBounds(0, 0, primaryExtent, height)
        } else {
            BubbleNodeBounds(width - primaryExtent, 0, width, height)
        }
    }

    val freeExtent = (width - primaryExtent).coerceAtLeast(1)
    val cancelExtent = minOf(freeExtent, (height * CANCEL_TO_IDLE_RATIO).roundToInt())
        .coerceAtLeast(1)
    return if (state.anchoredLeft) {
        BubbleNodeBounds(width - cancelExtent, 0, width, height)
    } else {
        BubbleNodeBounds(0, 0, cancelExtent, height)
    }
}

private const val CANCEL_TO_IDLE_RATIO = 48f / 78f
