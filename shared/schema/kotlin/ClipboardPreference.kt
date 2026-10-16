// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class ClipboardPreference(val wireValue: String) {
    PRESERVE("preserve"),
    COPY("copy");
    companion object {
        fun fromWire(value: String): ClipboardPreference? =
            entries.firstOrNull { it.wireValue == value }
    }
}
