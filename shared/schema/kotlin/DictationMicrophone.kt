// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class DictationMicrophone(val wireValue: String) {
    AUTHORIZED("authorized"),
    NOT_DETERMINED("notDetermined"),
    DENIED("denied"),
    RESTRICTED("restricted"),
    UNKNOWN("unknown");
    companion object {
        fun fromWire(value: String): DictationMicrophone? =
            entries.firstOrNull { it.wireValue == value }
    }
}
