// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState12(
    val `available`: List<BusinessCommandPass>,
    val `depositReceipts`: List<String>,
    val `issuance`: BusinessCommandJournal?,
    val `pending`: List<BusinessCommandLease2>,
    val `receipts`: List<BusinessCommandReceipt2>,
    val `retiring`: List<BusinessCommandPass>,
    val `scopeEpoch`: Long?
)
