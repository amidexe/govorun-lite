// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase6(val wireValue: String) {
    BEGIN("begin"),
    RECONCILE("reconcile"),
    REFRESH("refresh"),
    WARM("warm");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase6? =
            entries.firstOrNull { it.wireValue == value }
    }
}
