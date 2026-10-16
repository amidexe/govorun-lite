// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand4 { val `action`: String }
data class BusinessCommandCommand4_inspectKey(
    val `key`: BusinessCommandKey
) : BusinessCommandCommand4 { override val `action`: String = "inspectKey" }
data class BusinessCommandCommand4_prepareAndroid(
    val `count`: Int,
    val `expires_at_millis`: Long,
    val `key`: BusinessCommandKey,
    val `now_millis`: Long
) : BusinessCommandCommand4 { override val `action`: String = "prepareAndroid" }
data class BusinessCommandCommand4_restoreAndroid(
    val `key`: BusinessCommandKey,
    val `requests`: List<BusinessCommandMaterial>
) : BusinessCommandCommand4 { override val `action`: String = "restoreAndroid" }
data class BusinessCommandCommand4_finalizeAndroid(
    val `key`: BusinessCommandKey,
    val `requests`: List<BusinessCommandMaterial>,
    val `signatures`: List<String>
) : BusinessCommandCommand4 { override val `action`: String = "finalizeAndroid" }
data class BusinessCommandCommand4_prepare(
    val `count`: Int,
    val `expires_at_millis`: Long,
    val `key`: BusinessCommandKey,
    val `now_millis`: Long
) : BusinessCommandCommand4 { override val `action`: String = "prepare" }
data class BusinessCommandCommand4_restore(
    val `requests`: List<BusinessCommandRequestSnapshot>
) : BusinessCommandCommand4 { override val `action`: String = "restore" }
data class BusinessCommandCommand4_finalize(
    val `requests`: List<BusinessCommandRequestSnapshot>,
    val `signatures`: List<String>
) : BusinessCommandCommand4 { override val `action`: String = "finalize" }
