// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class AccessInteractionAction(val wireValue: String) {
    TERMS("terms"),
    FREE_ACCESS("freeAccess"),
    RESUME_FREE_ACCESS("resumeFreeAccess"),
    ENABLE("enable");
    companion object {
        fun fromWire(value: String): AccessInteractionAction? =
            entries.firstOrNull { it.wireValue == value }
    }
}
