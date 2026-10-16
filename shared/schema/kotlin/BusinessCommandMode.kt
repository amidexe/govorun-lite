// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandMode(val wireValue: String) {
    EVALUATING("evaluating"),
    PENDING("pending"),
    ENABLED("enabled"),
    LOCAL_ONLY("localOnly");
    companion object {
        fun fromWire(value: String): BusinessCommandMode? =
            entries.firstOrNull { it.wireValue == value }
    }
}
