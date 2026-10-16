// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class AboutSnapshot(
    val `buildID`: String,
    val `keyStorage`: String,
    val `licenses`: List<ApplicationLicense>,
    val `nativeBuildID`: String,
    val `nativeVersion`: String,
    val `platformBuild`: String,
    val `version`: String
)
