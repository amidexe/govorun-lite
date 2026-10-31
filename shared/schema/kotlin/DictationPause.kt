// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class DictationPause(val wireValue: String) {
    AUTO("auto"),
    SHORT("short"),
    MEDIUM("medium"),
    LONG("long");
    companion object {
        fun fromWire(value: String): DictationPause? =
            entries.firstOrNull { it.wireValue == value }
    }
}
