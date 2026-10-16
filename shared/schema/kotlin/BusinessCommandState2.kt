// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState2(
    val `activeId`: String?,
    val `instruction`: String,
    val `presets`: List<BusinessCommandStyle>,
    val `selectedStyle`: String
)
