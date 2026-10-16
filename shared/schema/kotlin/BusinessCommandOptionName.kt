// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandOptionName(val wireValue: String) {
    EMOJI_MODE("emojiMode"),
    REDUCED_PUNCTUATION("reducedPunctuation"),
    PROFANITY_FILTER("profanityFilter");
    companion object {
        fun fromWire(value: String): BusinessCommandOptionName? =
            entries.firstOrNull { it.wireValue == value }
    }
}
