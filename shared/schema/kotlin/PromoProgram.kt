// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class PromoProgram(
    val `accessDays`: Long,
    val `accessMode`: String,
    val `canActivate`: Boolean,
    val `dailyLimit`: Long,
    val `endsAt`: String?,
    val `enrollmentOpen`: Boolean,
    val `message`: String,
    val `participated`: Boolean,
    val `periodDays`: Long,
    val `periodLimit`: Long,
    val `programVersion`: String,
    val `state`: String,
    val `visible`: Boolean
)
