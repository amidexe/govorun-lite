// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandTrigger2(val wireValue: String) {
    MAINTENANCE("maintenance"),
    RECONNECT("reconnect"),
    DEMAND("demand");
    companion object {
        fun fromWire(value: String): BusinessCommandTrigger2? =
            entries.firstOrNull { it.wireValue == value }
    }
}
