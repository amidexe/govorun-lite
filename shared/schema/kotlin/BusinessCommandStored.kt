// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandStored(
    val `activeId`: String?,
    val `instruction`: String,
    val `legacyOptions`: BusinessCommandStoredOptions,
    val `optionsByStyle`: Map<String, BusinessCommandStoredOptions>,
    val `presets`: List<BusinessCommandStoredPreset>?,
    val `selectedStyle`: String?
)
