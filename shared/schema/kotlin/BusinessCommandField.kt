// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandField(val wireValue: String) {
    NAME("name"),
    INSTRUCTION("instruction");
    companion object {
        fun fromWire(value: String): BusinessCommandField? =
            entries.firstOrNull { it.wireValue == value }
    }
}
