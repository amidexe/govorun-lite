// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class DictationSnapshot(
    val `acceleration`: DictationAcceleration,
    val `alwaysOnMicrophone`: Boolean,
    val `durationMode`: DictationDuration,
    val `inputChannel`: Int?,
    val `inputChannels`: Int,
    val `launchAtLogin`: DictationLogin,
    val `markerMinimum`: Long,
    val `maxMinutes`: Int,
    val `microphoneId`: String?,
    val `microphones`: List<DictationMicrophoneDevice>,
    val `modelUnloadMinutes`: Int,
    val `muteAudio`: Boolean,
    val `pause`: DictationPause,
    val `permissions`: DictationPermissions,
    val `playSounds`: Boolean,
    val `pushToTalk`: Boolean,
    val `voiceMarker`: DictationMarker
)
