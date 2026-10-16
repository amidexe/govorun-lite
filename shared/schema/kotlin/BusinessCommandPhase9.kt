// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase9(val wireValue: String) {
    START("start"),
    REFRESHED("refreshed"),
    JOIN("join"),
    MAINTENANCE("maintenance"),
    AUTOMATIC("automatic");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase9? =
            entries.firstOrNull { it.wireValue == value }
    }
}
