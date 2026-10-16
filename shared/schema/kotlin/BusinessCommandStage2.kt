// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandStage2(val wireValue: String) {
    CONTENT("content"),
    PREFERENCES("preferences"),
    STATISTICS("statistics");
    companion object {
        fun fromWire(value: String): BusinessCommandStage2? =
            entries.firstOrNull { it.wireValue == value }
    }
}
