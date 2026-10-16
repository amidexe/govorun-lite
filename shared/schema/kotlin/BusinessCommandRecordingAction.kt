// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandRecordingAction(val wireValue: String) {
    READ("read"),
    ONBOARDING("onboarding"),
    SELECT("select");
    companion object {
        fun fromWire(value: String): BusinessCommandRecordingAction? =
            entries.firstOrNull { it.wireValue == value }
    }
}
