// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandFailure2(val wireValue: String) {
    NETWORK("network"),
    OFFLINE("offline"),
    SECURITY("security"),
    CANCELLED("cancelled"),
    DEADLINE("deadline"),
    OTHER("other");
    companion object {
        fun fromWire(value: String): BusinessCommandFailure2? =
            entries.firstOrNull { it.wireValue == value }
    }
}
