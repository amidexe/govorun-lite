// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class StatisticsMode(val wireValue: String) {
    EVALUATING("evaluating"),
    PENDING("pending"),
    ENABLED("enabled"),
    LOCAL_ONLY("localOnly");
    companion object {
        fun fromWire(value: String): StatisticsMode? =
            entries.firstOrNull { it.wireValue == value }
    }
}
