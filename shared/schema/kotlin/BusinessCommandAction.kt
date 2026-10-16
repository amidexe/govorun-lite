// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandAction { val `kind`: String }
data object BusinessCommandAction_inspect : BusinessCommandAction { override val `kind`: String = "inspect" }
data class BusinessCommandAction_countWords(
    val `text`: String
) : BusinessCommandAction { override val `kind`: String = "countWords" }
data class BusinessCommandAction_record(
    val `event`: BusinessCommandEvent2,
    val `today`: String
) : BusinessCommandAction { override val `kind`: String = "record" }
data class BusinessCommandAction_reset(
    val `account`: String?
) : BusinessCommandAction { override val `kind`: String = "reset" }
data class BusinessCommandAction_prepare(
    val `account`: String,
    val `today`: String
) : BusinessCommandAction { override val `kind`: String = "prepare" }
data class BusinessCommandAction_deleteAccepted(
    val `account`: String,
    val `epoch`: Long
) : BusinessCommandAction { override val `kind`: String = "deleteAccepted" }
data class BusinessCommandAction_accept(
    val `account`: String,
    val `receipt`: BusinessCommandReceipt,
    val `response`: BusinessCommandResponse
) : BusinessCommandAction { override val `kind`: String = "accept" }
