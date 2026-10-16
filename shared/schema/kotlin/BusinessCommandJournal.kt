// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandJournal(
    val `count`: Int,
    val `id`: String,
    val `receipts`: List<String>,
    val `scopeEpoch`: Long?
)
