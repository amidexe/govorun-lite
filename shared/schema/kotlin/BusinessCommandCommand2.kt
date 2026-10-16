// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand2 { val `kind`: String }
data object BusinessCommandCommand2_key : BusinessCommandCommand2 { override val `kind`: String = "key" }
data class BusinessCommandCommand2_issue(
    val `id`: String,
    val `key_id`: String,
    val `messages`: List<String>,
    val `receipts`: List<String>
) : BusinessCommandCommand2 { override val `kind`: String = "issue" }
data class BusinessCommandCommand2_deposit(
    val `id`: String,
    val `receipts`: List<String>,
    val `scope_id`: String
) : BusinessCommandCommand2 { override val `kind`: String = "deposit" }
data class BusinessCommandCommand2_depositResponse(
    val `body`: Any?,
    val `count`: Int
) : BusinessCommandCommand2 { override val `kind`: String = "depositResponse" }
data class BusinessCommandCommand2_rejectedDeposit(
    val `body`: Any?,
    val `status`: Int
) : BusinessCommandCommand2 { override val `kind`: String = "rejectedDeposit" }
