// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandTrigger(val wireValue: String) {
    MAINTENANCE("maintenance"),
    RECONNECT("reconnect"),
    DEMAND("demand");
    companion object {
        fun fromWire(value: String): BusinessCommandTrigger? =
            entries.firstOrNull { it.wireValue == value }
    }
}
