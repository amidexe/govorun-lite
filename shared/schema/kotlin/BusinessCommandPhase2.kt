// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandPhase2(val wireValue: String) {
    BEGIN("begin"),
    DRAFT("draft"),
    ROUTE("route"),
    OFFICIAL("official"),
    CONFIGURATION("configuration"),
    SELECTION("selection"),
    PREPARATION("preparation"),
    NETWORK("network"),
    RESPONSE("response"),
    DONE("done");
    companion object {
        fun fromWire(value: String): BusinessCommandPhase2? =
            entries.firstOrNull { it.wireValue == value }
    }
}
