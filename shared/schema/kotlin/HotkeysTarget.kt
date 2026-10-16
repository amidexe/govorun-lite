// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class HotkeysTarget(val wireValue: String) {
    DICTATION("dictation"),
    PLAIN("plain"),
    CANCEL("cancel");
    companion object {
        fun fromWire(value: String): HotkeysTarget? =
            entries.firstOrNull { it.wireValue == value }
    }
}
