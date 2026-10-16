package com.govorun.lite.model.settings

enum class UiThemeMode(val id: String) {
    LIGHT("light"),
    DARK("dark"),
    SYSTEM("system");

    companion object {
        fun fromId(id: String?): UiThemeMode =
            valueOf(com.govorun.lite.shared.SharedLocalModels.scalar("theme", id).uppercase())
    }
}
