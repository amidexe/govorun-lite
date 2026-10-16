// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BillingPromotionAccessKind(val wireValue: String) {
    DONE("done"),
    ACCEPT_TERMS("acceptTerms"),
    REFRESH("refresh");
    companion object {
        fun fromWire(value: String): BillingPromotionAccessKind? =
            entries.firstOrNull { it.wireValue == value }
    }
}
