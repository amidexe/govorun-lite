// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class ThemeMode(val wireValue: String) {
    SYSTEM("system"),
    LIGHT("light"),
    DARK("dark");
    companion object {
        fun fromWire(value: String): ThemeMode? =
            entries.firstOrNull { it.wireValue == value }
    }
}
