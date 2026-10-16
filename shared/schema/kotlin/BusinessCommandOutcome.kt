// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandOutcome(val wireValue: String) {
    READY("ready"),
    RETRY("retry"),
    FAILED("failed"),
    OFFLINE("offline"),
    CANCELLED("cancelled");
    companion object {
        fun fromWire(value: String): BusinessCommandOutcome? =
            entries.firstOrNull { it.wireValue == value }
    }
}
