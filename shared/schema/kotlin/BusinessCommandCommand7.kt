// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand7 { val `kind`: String }
data class BusinessCommandCommand7_activationSelection(
    val `available`: List<String>,
    val `first_activation`: Boolean,
    val `requested`: String?,
    val `selected`: String
) : BusinessCommandCommand7 { override val `kind`: String = "activationSelection" }
data class BusinessCommandCommand7_readStored(
    val `stored`: BusinessCommandStored
) : BusinessCommandCommand7 { override val `kind`: String = "readStored" }
data object BusinessCommandCommand7_limits : BusinessCommandCommand7 { override val `kind`: String = "limits" }
data class BusinessCommandCommand7_inspectText(
    val `field`: BusinessCommandField,
    val `value`: String
) : BusinessCommandCommand7 { override val `kind`: String = "inspectText" }
data class BusinessCommandCommand7_project(
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "project" }
data class BusinessCommandCommand7_save(
    val `id`: String?,
    val `instruction`: String,
    val `name`: String,
    val `new_id`: String,
    val `options`: BusinessCommandOptions?,
    val `select_saved`: Boolean,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "save" }
data class BusinessCommandCommand7_saveActive(
    val `current_options`: BusinessCommandOptions,
    val `instruction`: String,
    val `new_id`: String,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "saveActive" }
data class BusinessCommandCommand7_select(
    val `id`: String,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "select" }
data class BusinessCommandCommand7_delete(
    val `id`: String,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "delete" }
data class BusinessCommandCommand7_options(
    val `id`: String,
    val `options`: BusinessCommandOptions,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "options" }
data class BusinessCommandCommand7_replace(
    val `presets`: List<BusinessCommandStyle>,
    val `selected_id`: String,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "replace" }
data class BusinessCommandCommand7_choices(
    val `context`: BusinessCommandContext
) : BusinessCommandCommand7 { override val `kind`: String = "choices" }
data class BusinessCommandCommand7_activationOptions(
    val `context`: BusinessCommandContext,
    val `facts`: BusinessCommandFacts,
    val `supplied`: BusinessCommandSelectionOptions?
) : BusinessCommandCommand7 { override val `kind`: String = "activationOptions" }
data class BusinessCommandCommand7_selectStyle(
    val `activation_options`: BusinessCommandSelectionOptions?,
    val `context`: BusinessCommandContext,
    val `selection`: String
) : BusinessCommandCommand7 { override val `kind`: String = "selectStyle" }
data class BusinessCommandCommand7_setSelectedOption(
    val `context`: BusinessCommandContext,
    val `name`: BusinessCommandOptionName,
    val `value`: Any?
) : BusinessCommandCommand7 { override val `kind`: String = "setSelectedOption" }
data class BusinessCommandCommand7_detachApplicationReferences(
    val `id`: String,
    val `rules`: List<BusinessCommandApplicationRule>
) : BusinessCommandCommand7 { override val `kind`: String = "detachApplicationReferences" }
data class BusinessCommandCommand7_deleteWithReferences(
    val `id`: String,
    val `rules`: List<BusinessCommandApplicationRule>,
    val `state`: BusinessCommandState2
) : BusinessCommandCommand7 { override val `kind`: String = "deleteWithReferences" }
data class BusinessCommandCommand7_saveApplicationRule(
    val `context`: BusinessCommandContext,
    val `rule`: BusinessCommandApplicationRule,
    val `rules`: List<BusinessCommandApplicationRule>
) : BusinessCommandCommand7 { override val `kind`: String = "saveApplicationRule" }
data class BusinessCommandCommand7_deleteApplicationRule(
    val `confirmed`: Boolean,
    val `context`: BusinessCommandContext,
    val `id`: String,
    val `rules`: List<BusinessCommandApplicationRule>
) : BusinessCommandCommand7 { override val `kind`: String = "deleteApplicationRule" }
