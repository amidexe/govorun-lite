package com.govorun.lite.stylization

/**
 * Process-local signal for the custom-rule editor. A dictated draft must be
 * inserted verbatim: applying the previously saved custom rule to the draft
 * makes it look as if the unsaved text executed itself.
 */
internal object CustomStyleDraftSession {
    @Volatile
    private var active = false

    fun setActive(value: Boolean) {
        active = value
    }

    fun isActive(): Boolean = active
}
