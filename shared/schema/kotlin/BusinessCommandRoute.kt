// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandRoute(val wireValue: String) {
    LOCAL("local"),
    OFFICIAL("official"),
    SELF_HOSTED("selfHosted");
    companion object {
        fun fromWire(value: String): BusinessCommandRoute? =
            entries.firstOrNull { it.wireValue == value }
    }
}
