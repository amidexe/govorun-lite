// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase3(val wireValue: String) {
    READY("ready"),
    AWAITING_RESPONSE("awaitingResponse"),
    RECEIVED("received"),
    DONE("done");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase3? =
            entries.firstOrNull { it.wireValue == value }
    }
}
