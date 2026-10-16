// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class DictationDuration(val wireValue: String) {
    LOCAL("local"),
    STYLES("styles"),
    UNLIMITED("unlimited");
    companion object {
        fun fromWire(value: String): DictationDuration? =
            entries.firstOrNull { it.wireValue == value }
    }
}
