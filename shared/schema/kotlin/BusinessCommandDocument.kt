// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandDocument(
    val `customStyles`: List<BusinessCommandStyle>,
    val `dictionary`: BusinessCommandDictionary,
    val `schemaVersion`: Int,
    val `selectedCustomStyleId`: String
)
