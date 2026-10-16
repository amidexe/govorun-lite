// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase7(val wireValue: String) {
    IDLE("idle"),
    RUNNING("running"),
    WAITING("waiting");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase7? =
            entries.firstOrNull { it.wireValue == value }
    }
}
