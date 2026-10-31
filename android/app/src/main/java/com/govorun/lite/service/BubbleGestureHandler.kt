package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.annotation.SuppressLint
import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.util.Prefs
import kotlin.math.abs

/** Pointer gestures for the fixed surface; accessibility actions live in [BubbleInputView]. */
internal class BubbleGestureHandler(
    private val service: AccessibilityService,
    private val windowManager: WindowManager,
    private val callbacks: BubbleOverlayController.Callbacks,
) {
    @SuppressLint("ClickableViewAccessibility")
    fun install(
        input: View,
        control: BubbleView,
        inputParams: WindowManager.LayoutParams,
        visualParams: WindowManager.LayoutParams,
    ) {
        val dragThresholdPx = DRAG_THRESHOLD_DP * service.resources.displayMetrics.density
        input.setOnTouchListener(object : View.OnTouchListener {
            private var initialY = 0
            private var initialTouchY = 0f
            private var initialTouchX = 0f
            private var dragged = false
            private var holdStarted = false
            private var pressedAction = BubbleView.Action.NONE
            private val holdHandler = Handler(Looper.getMainLooper())
            private val holdRunnable = Runnable {
                if (dragged || holdStarted) return@Runnable
                if (!input.isAttachedToWindow || !input.isShown) {
                    callbacks.onPointerCancel()
                    return@Runnable
                }
                // A finger trembles a few dp while it holds: only a drag (past DRAG_THRESHOLD_DP, cancelled in
                // ACTION_MOVE) is not a hold. A 5 dp limit here turned every other hold into a tap.
                holdStarted = true
                callbacks.onHoldStart()
            }

            override fun onTouch(v: View, event: MotionEvent): Boolean {
                if (control.isProcessing()) return true
                if (control.isConfirmationRecording()) return handleConfirmation(event)

                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        initialY = inputParams.y
                        initialTouchY = event.rawY
                        initialTouchX = event.rawX
                        dragged = false
                        holdStarted = false
                        callbacks.onPointerDown()
                        holdHandler.postDelayed(holdRunnable, HOLD_DELAY_MS)
                    }
                    MotionEvent.ACTION_MOVE -> {
                        if (holdStarted) return true
                        val dy = event.rawY - initialTouchY
                        val dx = event.rawX - initialTouchX
                        if (abs(dy) > dragThresholdPx || abs(dx) > dragThresholdPx) {
                            if (!dragged) {
                                dragged = true
                                callbacks.onPointerCancel()
                                holdHandler.removeCallbacks(holdRunnable)
                            }
                            if (abs(dy) > dragThresholdPx &&
                                !Prefs.isBubblePositionLocked(service)
                            ) {
                                moveTo(input, control, inputParams, visualParams, initialY + dy.toInt())
                            }
                        }
                    }
                    MotionEvent.ACTION_UP -> {
                        holdHandler.removeCallbacks(holdRunnable)
                        when {
                            dragged -> Prefs.setBubbleY(service, inputParams.y)
                            holdStarted -> callbacks.onHoldStop(silent = false)
                            else -> callbacks.onTap()
                        }
                        callbacks.onPointerCancel()
                    }
                    MotionEvent.ACTION_CANCEL -> {
                        callbacks.onPointerCancel()
                        holdHandler.removeCallbacks(holdRunnable)
                        if (holdStarted) callbacks.onHoldStop(silent = true)
                        if (dragged) Prefs.setBubbleY(service, inputParams.y)
                    }
                    else -> return false
                }
                return true
            }

            private fun handleConfirmation(event: MotionEvent): Boolean {
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        pressedAction = control.actionAtControl(event.x, event.y)
                    }
                    MotionEvent.ACTION_UP -> {
                        val releasedAction = control.actionAtControl(event.x, event.y)
                        if (pressedAction == releasedAction) {
                            when (releasedAction) {
                                BubbleView.Action.CONFIRM -> callbacks.onConfirm()
                                BubbleView.Action.CANCEL -> callbacks.onCancel()
                                BubbleView.Action.NONE -> Unit
                            }
                        }
                        pressedAction = BubbleView.Action.NONE
                    }
                    MotionEvent.ACTION_CANCEL -> pressedAction = BubbleView.Action.NONE
                }
                return true
            }
        })
    }

    private fun moveTo(
        input: View,
        control: BubbleView,
        inputParams: WindowManager.LayoutParams,
        visualParams: WindowManager.LayoutParams,
        y: Int,
    ) {
        inputParams.y = y
        visualParams.y = y
        try { windowManager.updateViewLayout(input, inputParams) } catch (_: Exception) {}
        try { windowManager.updateViewLayout(control, visualParams) } catch (_: Exception) {}
    }

    private companion object {
        const val DRAG_THRESHOLD_DP = 10f
        const val HOLD_DELAY_MS = 250L
    }
}
