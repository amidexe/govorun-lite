// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent14 { val `kind`: String }
data object BusinessCommandEvent14_validate : BusinessCommandEvent14 { override val `kind`: String = "validate" }
data class BusinessCommandEvent14_record(
    val `day`: String?,
    val `scopes`: List<String?>
) : BusinessCommandEvent14 { override val `kind`: String = "record" }
data class BusinessCommandEvent14_begin(
    val `admitted_scope`: String?,
    val `day`: String?,
    val `lease_id`: String,
    val `scopes`: List<String?>
) : BusinessCommandEvent14 { override val `kind`: String = "begin" }
data class BusinessCommandEvent14_settle(
    val `consumed`: Boolean,
    val `lease_id`: String
) : BusinessCommandEvent14 { override val `kind`: String = "settle" }
data class BusinessCommandEvent14_observe(
    val `device_days`: Map<String, Int>,
    val `device_requests`: Map<String, BusinessCommandRequestCount>,
    val `protected_scopes`: List<String>,
    val `scope`: BusinessCommandScope3
) : BusinessCommandEvent14 { override val `kind`: String = "observe" }
data class BusinessCommandEvent14_acknowledge(
    val `days`: List<BusinessCommandDay2>,
    val `report`: BusinessCommandReport
) : BusinessCommandEvent14 { override val `kind`: String = "acknowledge" }
data class BusinessCommandEvent14_inspect(
    val `scope`: BusinessCommandScope3
) : BusinessCommandEvent14 { override val `kind`: String = "inspect" }
data class BusinessCommandEvent14_outstanding(
    val `day`: String?,
    val `scope_id`: String
) : BusinessCommandEvent14 { override val `kind`: String = "outstanding" }
