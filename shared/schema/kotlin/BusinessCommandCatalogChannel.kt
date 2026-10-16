// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandCatalogChannel(val wireValue: String) {
    DIRECT("direct");
    companion object {
        fun fromWire(value: String): BusinessCommandCatalogChannel? =
            entries.firstOrNull { it.wireValue == value }
    }
}
