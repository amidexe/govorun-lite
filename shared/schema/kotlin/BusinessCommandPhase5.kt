// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase5(val wireValue: String) {
    STARTING("starting"),
    ACCEPTED("accepted"),
    WAITING("waiting");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase5? =
            entries.firstOrNull { it.wireValue == value }
    }
}
