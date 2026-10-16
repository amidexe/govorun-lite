// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandStage(val wireValue: String) {
    READ("read"),
    WRITE("write");
    companion object {
        fun fromWire(value: String): BusinessCommandStage? =
            entries.firstOrNull { it.wireValue == value }
    }
}
