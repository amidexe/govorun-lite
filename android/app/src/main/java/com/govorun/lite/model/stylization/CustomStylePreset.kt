package com.govorun.lite.model.stylization

data class CustomStylePreset(
    val id: String,
    val name: String,
    val instruction: String,
    val punctuationMode: PunctuationMode = PunctuationMode.NORMAL,
    val emojiEnabled: Boolean = false,
    val profanityFilterEnabled: Boolean = false,
    val emojiMode: EmojiMode = EmojiMode.fromApiId(null, emojiEnabled),
) {
    val resolvedEmojiMode: EmojiMode
        get() = EmojiMode.fromApiId(com.govorun.lite.shared.SharedLocalModels.scalar("resolvedEmoji", emojiMode.apiId, emojiEnabled))
}

