// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class AccountSnapshot(
    val `busy`: Boolean,
    val `deviceLimit`: Int,
    val `devices`: List<AccountDevice>,
    val `displayName`: String?,
    val `messageCode`: String?,
    val `providerNoticeCode`: String?,
    val `providers`: List<AccountProvider>,
    val `replacingDevice`: Boolean,
    val `scope`: String,
    val `status`: String,
    val `storage`: AccountStorage,
    val `syncMode`: String
)
