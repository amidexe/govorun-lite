// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class DictationLogin(val wireValue: String) {
    ENABLED("enabled"),
    DISABLED("disabled"),
    APPROVAL_REQUIRED("approvalRequired"),
    UNAVAILABLE("unavailable");
    companion object {
        fun fromWire(value: String): DictationLogin? =
            entries.firstOrNull { it.wireValue == value }
    }
}
