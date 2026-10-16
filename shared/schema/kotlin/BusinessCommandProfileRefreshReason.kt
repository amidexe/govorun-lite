// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandProfileRefreshReason(val wireValue: String) {
    EXPLICIT("explicit"),
    BACKGROUND("background"),
    FOCUS("focus"),
    SYNC("sync");
    companion object {
        fun fromWire(value: String): BusinessCommandProfileRefreshReason? =
            entries.firstOrNull { it.wireValue == value }
    }
}
