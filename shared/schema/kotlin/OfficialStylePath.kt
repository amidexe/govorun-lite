// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class OfficialStylePath(val wireValue: String) {
    V2_STYLIZE("/v2/stylize"),
    V1_STYLIZE_STATUS("/v1/stylize-status"),
    V1_TEXT_CREDITS_RECONCILE("/v1/text-credits/reconcile"),
    V1_TEXT_CREDITS_RETIRE("/v1/text-credits/retire"),
    V1_AVAILABILITY("/v1/availability"),
    V1_CLIENT_OUTCOMES("/v1/client-outcomes"),
    V1_CLIENT_RELIABILITY("/v1/client-reliability");
    companion object {
        fun fromWire(value: String): OfficialStylePath? =
            entries.firstOrNull { it.wireValue == value }
    }
}
