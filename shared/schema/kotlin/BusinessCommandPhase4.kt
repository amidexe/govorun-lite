// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase4(val wireValue: String) {
    START("start"),
    STATUS("status");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase4? =
            entries.firstOrNull { it.wireValue == value }
    }
}
