// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent13 { val `kind`: String }
data object BusinessCommandEvent13_normalize : BusinessCommandEvent13 { override val `kind`: String = "normalize" }
data class BusinessCommandEvent13_changeScope(
    val `compatible`: Boolean
) : BusinessCommandEvent13 { override val `kind`: String = "changeScope" }
data class BusinessCommandEvent13_lease(
    val `count`: Int,
    val `id`: String
) : BusinessCommandEvent13 { override val `kind`: String = "lease" }
data class BusinessCommandEvent13_settle(
    val `id`: String,
    val `restore`: Boolean
) : BusinessCommandEvent13 { override val `kind`: String = "settle" }
data class BusinessCommandEvent13_beginIssuance(
    val `journal`: BusinessCommandJournal
) : BusinessCommandEvent13 { override val `kind`: String = "beginIssuance" }
data class BusinessCommandEvent13_finishIssuance(
    val `id`: String,
    val `passes`: List<BusinessCommandPass>
) : BusinessCommandEvent13 { override val `kind`: String = "finishIssuance" }
data class BusinessCommandEvent13_discardIssuance(
    val `discard_receipts`: Boolean,
    val `id`: String,
    val `scope_epoch`: Long?
) : BusinessCommandEvent13 { override val `kind`: String = "discardIssuance" }
data class BusinessCommandEvent13_selectReceipts(
    val `count`: Int
) : BusinessCommandEvent13 { override val `kind`: String = "selectReceipts" }
data class BusinessCommandEvent13_retire(
    val `discard`: Boolean,
    val `receipts`: List<BusinessCommandReceipt2>,
    val `scope_epoch`: Long,
    val `selected`: List<String>
) : BusinessCommandEvent13 { override val `kind`: String = "retire" }
data object BusinessCommandEvent13_returnUnused : BusinessCommandEvent13 { override val `kind`: String = "returnUnused" }
data object BusinessCommandEvent13_selectDeposit : BusinessCommandEvent13 { override val `kind`: String = "selectDeposit" }
data class BusinessCommandEvent13_completeDeposit(
    val `receipts`: List<String>,
    val `scope_epoch`: Long
) : BusinessCommandEvent13 { override val `kind`: String = "completeDeposit" }
