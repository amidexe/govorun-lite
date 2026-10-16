package com.govorun.lite.model.stylization

enum class TextStyle(val apiId: String) {
    FORMAL("formal"), LIGHT("light"), CONCISE("concise"), CUSTOM("custom");

    companion object {
        fun fromApiId(value: String?): TextStyle = valueOf(com.govorun.lite.shared.SharedLocalModels.scalar("style", value).uppercase())
    }
}
