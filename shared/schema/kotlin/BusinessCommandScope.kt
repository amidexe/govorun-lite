// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandScope(
    val `access`: Any?,
    val `accountId`: String?,
    val `consentActive`: Boolean,
    val `consentVersion`: Int,
    val `generation`: Long,
    val `signedIn`: Boolean,
    val `terms`: Int
)
