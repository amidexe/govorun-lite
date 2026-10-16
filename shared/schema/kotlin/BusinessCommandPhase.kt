// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase(val wireValue: String) {
    READ("read"),
    PATCH("patch");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase? =
            entries.firstOrNull { it.wireValue == value }
    }
}
