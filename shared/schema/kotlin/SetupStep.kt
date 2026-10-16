// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class SetupStep(val wireValue: String) {
    WELCOME("welcome"),
    DICTATION("dictation"),
    TRIAL("trial"),
    DONE("done");
    companion object {
        fun fromWire(value: String): SetupStep? =
            entries.firstOrNull { it.wireValue == value }
    }
}
