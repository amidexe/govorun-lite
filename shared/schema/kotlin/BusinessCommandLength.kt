// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandLength(val wireValue: String) {
    ELIGIBLE("eligible"),
    TOO_SHORT("tooShort"),
    TOO_LONG("tooLong"),
    INVALID("invalid");
    companion object {
        fun fromWire(value: String): BusinessCommandLength? =
            entries.firstOrNull { it.wireValue == value }
    }
}
