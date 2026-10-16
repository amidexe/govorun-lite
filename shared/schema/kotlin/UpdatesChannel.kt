// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class UpdatesChannel(val wireValue: String) {
    STABLE("stable"),
    TEST("test");
    companion object {
        fun fromWire(value: String): UpdatesChannel? =
            entries.firstOrNull { it.wireValue == value }
    }
}
