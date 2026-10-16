// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class SelfHostKeyState(val wireValue: String) {
    MISSING("missing"),
    SAVED("saved"),
    INACCESSIBLE("inaccessible"),
    STALE_REFERENCE("staleReference");
    companion object {
        fun fromWire(value: String): SelfHostKeyState? =
            entries.firstOrNull { it.wireValue == value }
    }
}
