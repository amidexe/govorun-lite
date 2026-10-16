// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent12 { val `kind`: String }
data class BusinessCommandEvent12_batch(
    val `count`: Int
) : BusinessCommandEvent12 { override val `kind`: String = "batch" }
data class BusinessCommandEvent12_operation(
    val `id`: String
) : BusinessCommandEvent12 { override val `kind`: String = "operation" }
data class BusinessCommandEvent12_lease(
    val `count`: Int,
    val `day`: String?,
    val `scope`: String?
) : BusinessCommandEvent12 { override val `kind`: String = "lease" }
data class BusinessCommandEvent12_replay(
    val `now_millis`: Long,
    val `replay_until_millis`: Long
) : BusinessCommandEvent12 { override val `kind`: String = "replay" }
data class BusinessCommandEvent12_response(
    val `expected_key_id`: String,
    val `key_id`: String,
    val `request_count`: Int,
    val `signature_count`: Int
) : BusinessCommandEvent12 { override val `kind`: String = "response" }
