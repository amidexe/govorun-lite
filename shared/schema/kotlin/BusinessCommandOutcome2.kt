// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandOutcome2(val wireValue: String) {
    READY("ready"),
    RETRY("retry"),
    TERMINAL("terminal"),
    OFFLINE("offline"),
    BLOCKED("blocked"),
    CANCELLED("cancelled"),
    HTTP_FAILURE("httpFailure"),
    PROGRESS("progress");
    companion object {
        fun fromWire(value: String): BusinessCommandOutcome2? =
            entries.firstOrNull { it.wireValue == value }
    }
}
