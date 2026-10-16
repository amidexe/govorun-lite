// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class UpdateSnapshot(
    val `channel`: UpdatesChannel,
    val `channelSelectable`: Boolean,
    val `dismissed`: Boolean,
    val `error`: String?,
    val `release`: UpdatesRelease?,
    val `status`: String,
    val `transfer`: UpdatesUpdateTransfer?
)
