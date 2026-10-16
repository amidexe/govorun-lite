package com.govorun.lite.model.stylization

enum class PunctuationMode(val apiId: String) {
    LESS("less"),
    NORMAL("normal");

    companion object {
        // Early 2.0 builds used to persist "more". It was never a useful
        // product option, so unknown and retired values deliberately become
        // the normal mode instead of changing the user's text unexpectedly.
        fun fromApiId(value: String?): PunctuationMode =
            valueOf(com.govorun.lite.shared.SharedLocalModels.scalar("punctuation", value).uppercase())
    }
}

enum class EmojiMode(val apiId: String) {
    OFF("off"),
    RESTRAINED("restrained"),
    LIVELY("lively"),
    BRIGHT("bright");

    val enabled: Boolean get() = this != OFF

    companion object {
        fun fromApiId(value: String?, legacyEnabled: Boolean = false): EmojiMode =
            valueOf(com.govorun.lite.shared.SharedLocalModels.scalar("emoji", value, legacyEnabled).uppercase())
    }
}

data class StyleOptions(
    val punctuation: PunctuationMode = PunctuationMode.NORMAL,
    val emojiEnabled: Boolean = false,
    val profanityFilterEnabled: Boolean = false,
    val emojiMode: EmojiMode = EmojiMode.fromApiId(null, emojiEnabled),
) {
    val resolvedEmojiMode: EmojiMode
        get() = EmojiMode.fromApiId(com.govorun.lite.shared.SharedLocalModels.scalar("resolvedEmoji", emojiMode.apiId, emojiEnabled))

    fun withEmojiMode(value: EmojiMode): StyleOptions = copy(
        emojiEnabled = value.enabled,
        emojiMode = value,
    )
}

