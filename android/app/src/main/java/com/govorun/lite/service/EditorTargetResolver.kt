package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityNodeInfo
import android.view.inputmethod.EditorInfo

internal fun resolveCurrentEditorInfo(
    service: AccessibilityService,
    inputMethodBridge: InputMethodBridge,
): EditorInfo? {
    inputMethodBridge.currentEditorInfo()?.let { return it }
    val focusedNode = try {
        service.rootInActiveWindow
            ?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            ?.takeIf { it.isEditable }
    } catch (_: Exception) {
        null
    }
    return InputFieldFilter.editorInfoFromNode(focusedNode)
}
