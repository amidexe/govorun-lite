// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandFacts4(
    val `incomingIdentityKey`: String?,
    val `incomingScopeKey`: String?,
    val `invalidateScope`: Boolean,
    val `previousIdentityKey`: String,
    val `previousScopeKey`: String,
    val `replaceIdentity`: Boolean
)
