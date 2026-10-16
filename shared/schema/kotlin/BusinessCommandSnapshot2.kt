// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandSnapshot2(
    val `authority`: BusinessCommandAuthority,
    val `logoutPending`: Boolean,
    val `profileFresh`: Boolean,
    val `sessionGeneration`: Long,
    val `signedIn`: Boolean,
    val `storageAvailable`: Boolean
)
