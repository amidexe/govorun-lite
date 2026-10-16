// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class HotkeySnapshot(
    val `canCapture`: Boolean,
    val `cancel`: List<String>,
    val `captureID`: String?,
    val `dictation`: List<String>,
    val `inputMode`: String,
    val `message`: String?,
    val `plain`: List<String>,
    val `preview`: List<String>,
    val `target`: HotkeysTarget?
)
