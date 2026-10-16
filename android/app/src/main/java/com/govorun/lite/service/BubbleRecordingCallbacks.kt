package com.govorun.lite.service

internal fun bubbleRecordingCallbacks(
    onTap: () -> Unit,
    onHoldStart: () -> Unit,
    onHoldStop: (Boolean) -> Unit,
    onConfirm: () -> Unit,
    onCancel: () -> Unit,
    isVadActive: () -> Boolean,
    onPointerDown: () -> Unit = {},
    onPointerCancel: () -> Unit = {},
): BubbleOverlayController.Callbacks = object : BubbleOverlayController.Callbacks {
    override fun onPointerDown() = onPointerDown.invoke()
    override fun onPointerCancel() = onPointerCancel.invoke()
    override fun onTap() = onTap.invoke()
    override fun onHoldStart() = onHoldStart.invoke()
    override fun onHoldStop(silent: Boolean) = onHoldStop.invoke(silent)
    override fun onConfirm() = onConfirm.invoke()
    override fun onCancel() = onCancel.invoke()
    override fun isVadActive(): Boolean = isVadActive.invoke()
}
