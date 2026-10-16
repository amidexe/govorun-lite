// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class DictationMarker(val wireValue: String) {
    OFF("off"),
    MICROPHONE("microphone"),
    TEXT("text");
    companion object {
        fun fromWire(value: String): DictationMarker? =
            entries.firstOrNull { it.wireValue == value }
    }
}
