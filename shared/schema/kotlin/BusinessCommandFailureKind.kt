// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandFailureKind(val wireValue: String) {
    IGNORED("ignored"),
    CANCELLED("cancelled"),
    SECURITY("security"),
    OFFLINE("offline"),
    TRANSPORT("transport"),
    DEADLINE("deadline"),
    HTTP("http"),
    SERVICE("service");
    companion object {
        fun fromWire(value: String): BusinessCommandFailureKind? =
            entries.firstOrNull { it.wireValue == value }
    }
}
