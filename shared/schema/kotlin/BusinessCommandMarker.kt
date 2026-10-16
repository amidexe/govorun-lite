// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandMarker(val wireValue: String) {
    OFF("off"),
    MICROPHONE("microphone"),
    TEXT("text"),
    LEGACY_TEXT("legacy_text");
    companion object {
        fun fromWire(value: String): BusinessCommandMarker? =
            entries.firstOrNull { it.wireValue == value }
    }
}
