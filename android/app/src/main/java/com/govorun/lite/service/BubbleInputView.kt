package com.govorun.lite.service

import android.content.Context
import android.graphics.Color
import android.graphics.Rect
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.accessibility.AccessibilityEvent
import android.widget.Button
import android.widget.TextView
import androidx.core.view.ViewCompat
import androidx.core.view.accessibility.AccessibilityNodeInfoCompat
import androidx.customview.widget.ExploreByTouchHelper
import com.govorun.lite.overlay.BubbleView

/**
 * Transparent input surface paired with the separately rendered [BubbleView].
 *
 * Pointer gestures continue through [BubbleGestureHandler]. Screen readers and
 * switch access receive virtual descendants with real button semantics instead
 * of one coordinate-only host node covering the complete overlay window.
 */
@Suppress("DEPRECATION")
internal class BubbleInputView(
    context: Context,
    private val callbacks: BubbleOverlayController.Callbacks,
) : View(context) {
    private var state: BubbleInputState? = null
    private val accessibilityHelper = BubbleExploreByTouchHelper()

    init {
        setBackgroundColor(Color.TRANSPARENT)
        contentDescription = null
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_YES
        ViewCompat.setAccessibilityDelegate(this, accessibilityHelper)
    }

    fun updateState(next: BubbleInputState) {
        val previous = state
        if (previous == next) return
        clearFocusForRemovedNode(next)
        state = next
        accessibilityHelper.invalidateRoot()
        if (previous != null && previous.presentation != next.presentation) {
            announceStatus(next)
        }
    }

    private fun announceStatus(next: BubbleInputState) {
        val status = bubbleAccessibilitySpecs(next.presentation)
            .singleOrNull { it.node == BubbleAccessibilityNode.STATUS }
            ?: return
        post {
            if (state == next && isShown) {
                announceForAccessibility(context.getText(status.label))
            }
        }
    }

    override fun dispatchHoverEvent(event: MotionEvent): Boolean =
        accessibilityHelper.dispatchHoverEvent(event) || super.dispatchHoverEvent(event)

    override fun dispatchKeyEvent(event: KeyEvent): Boolean =
        accessibilityHelper.dispatchKeyEvent(event) || super.dispatchKeyEvent(event)

    override fun onFocusChanged(
        gainFocus: Boolean,
        direction: Int,
        previouslyFocusedRect: Rect?,
    ) {
        super.onFocusChanged(gainFocus, direction, previouslyFocusedRect)
        accessibilityHelper.onFocusChanged(gainFocus, direction, previouslyFocusedRect)
    }

    private fun clearFocusForRemovedNode(next: BubbleInputState) {
        val visibleIds = bubbleAccessibilitySpecs(next.presentation)
            .mapTo(mutableSetOf()) { it.node.virtualId }
        accessibilityHelper.accessibilityFocusedVirtualViewId
            .takeUnless { it == ExploreByTouchHelper.INVALID_ID || it in visibleIds }
            ?.let { focused ->
                accessibilityHelper.getAccessibilityNodeProvider(this)?.performAction(
                    focused,
                    AccessibilityNodeInfoCompat.ACTION_CLEAR_ACCESSIBILITY_FOCUS,
                    null,
                )
            }
        accessibilityHelper.keyboardFocusedVirtualViewId
            .takeUnless { it == ExploreByTouchHelper.INVALID_ID || it in visibleIds }
            ?.let(accessibilityHelper::clearKeyboardFocusForVirtualView)
    }

    private fun dispatch(command: BubbleAccessibilityCommand) {
        when (command) {
            BubbleAccessibilityCommand.TAP -> callbacks.onTap()
            BubbleAccessibilityCommand.START_HOLD -> callbacks.onHoldStart()
            BubbleAccessibilityCommand.STOP_HOLD -> callbacks.onHoldStop(silent = false)
            BubbleAccessibilityCommand.CONFIRM -> callbacks.onConfirm()
            BubbleAccessibilityCommand.CANCEL -> callbacks.onCancel()
        }
    }

    private inner class BubbleExploreByTouchHelper : ExploreByTouchHelper(this@BubbleInputView) {
        override fun getVirtualViewAt(x: Float, y: Float): Int {
            val current = state ?: return INVALID_ID
            return bubbleAccessibilitySpecs(current.presentation)
                .firstOrNull { spec ->
                    bubbleAccessibilityBounds(current, spec.node).contains(x, y)
                }
                ?.node
                ?.virtualId
                ?: INVALID_ID
        }

        override fun getVisibleVirtualViews(virtualViewIds: MutableList<Int>) {
            val current = state ?: return
            bubbleAccessibilitySpecs(current.presentation)
                .mapTo(virtualViewIds) { it.node.virtualId }
        }

        override fun onPopulateNodeForHost(node: AccessibilityNodeInfoCompat) {
            super.onPopulateNodeForHost(node)
            node.contentDescription = null
            node.isClickable = false
            node.isFocusable = false
        }

        override fun onPopulateNodeForVirtualView(
            virtualViewId: Int,
            node: AccessibilityNodeInfoCompat,
        ) {
            val current = state
            val spec = specFor(virtualViewId)
            if (current == null || spec == null) {
                node.contentDescription = " "
                node.setBoundsInParent(Rect(0, 0, 1, 1))
                node.isVisibleToUser = false
                return
            }
            val bounds = bubbleAccessibilityBounds(current, spec.node)
            node.contentDescription = context.getString(spec.label)
            node.className = if (spec.click == null) {
                TextView::class.java.name
            } else {
                Button::class.java.name
            }
            node.setBoundsInParent(Rect(bounds.left, bounds.top, bounds.right, bounds.bottom))
            node.isEnabled = this@BubbleInputView.isEnabled
            node.isFocusable = true
            if (spec.click != null && this@BubbleInputView.isEnabled) {
                node.isClickable = true
                node.addAction(AccessibilityNodeInfoCompat.ACTION_CLICK)
            }
            spec.actions.forEach { action ->
                node.addAction(
                    AccessibilityNodeInfoCompat.AccessibilityActionCompat(
                        action.id,
                        context.getString(action.label),
                    ),
                )
            }
        }

        override fun onPerformActionForVirtualView(
            virtualViewId: Int,
            action: Int,
            arguments: Bundle?,
        ): Boolean {
            if (!this@BubbleInputView.isEnabled) return false
            val spec = specFor(virtualViewId) ?: return false
            val command = when (action) {
                AccessibilityNodeInfoCompat.ACTION_CLICK -> spec.click
                else -> spec.actions.firstOrNull { it.id == action }?.command
            } ?: return false
            sendEventForVirtualView(virtualViewId, AccessibilityEvent.TYPE_VIEW_CLICKED)
            dispatch(command)
            return true
        }

        private fun specFor(virtualViewId: Int): BubbleAccessibilitySpec? {
            val current = state ?: return null
            return bubbleAccessibilitySpecs(current.presentation)
                .firstOrNull { it.node.virtualId == virtualViewId }
        }
    }
}
