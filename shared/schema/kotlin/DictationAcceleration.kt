// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class DictationAcceleration(val wireValue: String) {
    AUTOMATIC("automatic"),
    CPU("cpu"),
    GPU("gpu");
    companion object {
        fun fromWire(value: String): DictationAcceleration? =
            entries.firstOrNull { it.wireValue == value }
    }
}
