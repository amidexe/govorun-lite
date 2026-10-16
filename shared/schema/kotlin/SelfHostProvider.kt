// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class SelfHostProvider(val wireValue: String) {
    OLLAMA("ollama"),
    OPENAI("openai"),
    GEMINI("gemini");
    companion object {
        fun fromWire(value: String): SelfHostProvider? =
            entries.firstOrNull { it.wireValue == value }
    }
}
