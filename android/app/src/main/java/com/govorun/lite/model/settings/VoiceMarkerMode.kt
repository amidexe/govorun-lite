package com.govorun.lite.model.settings

enum class VoiceMarkerMode(val storageId: String) {
    OFF("off"),
    MICROPHONE("microphone"),
    TEXT("text");

    companion object {
        fun fromStorageId(value: String?): VoiceMarkerMode =
            valueOf(com.govorun.lite.shared.SharedLocalModels.scalar("marker", value).uppercase())
    }
}
