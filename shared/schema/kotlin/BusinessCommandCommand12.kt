// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand12 { val `kind`: String }
data class BusinessCommandCommand12_parse(
    val `raw`: String
) : BusinessCommandCommand12 { override val `kind`: String = "parse" }
data class BusinessCommandCommand12_readStorage(
    val `stored`: Any?
) : BusinessCommandCommand12 { override val `kind`: String = "readStorage" }
data class BusinessCommandCommand12_writeStorage(
    val `enabled`: Boolean,
    val `raw`: String
) : BusinessCommandCommand12 { override val `kind`: String = "writeStorage" }
data class BusinessCommandCommand12_importStable(
    val `enabled`: Boolean,
    val `entries`: Any?,
    val `portable`: Any?
) : BusinessCommandCommand12 { override val `kind`: String = "importStable" }
data class BusinessCommandCommand12_edit(
    val `entries`: List<BusinessCommandEntry>,
    val `raw`: String
) : BusinessCommandCommand12 { override val `kind`: String = "edit" }
data class BusinessCommandCommand12_apply(
    val `enabled`: Boolean,
    val `raw`: String,
    val `text`: String
) : BusinessCommandCommand12 { override val `kind`: String = "apply" }
data class BusinessCommandCommand12_normalize(
    val `text`: String
) : BusinessCommandCommand12 { override val `kind`: String = "normalize" }
data class BusinessCommandCommand12_variants(
    val `text`: String
) : BusinessCommandCommand12 { override val `kind`: String = "variants" }
data class BusinessCommandCommand12_appendVariants(
    val `correct`: String,
    val `existing`: List<String>,
    val `recognized`: String
) : BusinessCommandCommand12 { override val `kind`: String = "appendVariants" }
data class BusinessCommandCommand12_voiceMappings(
    val `captured`: List<String>,
    val `correct`: String,
    val `raw`: String
) : BusinessCommandCommand12 { override val `kind`: String = "voiceMappings" }
data class BusinessCommandCommand12_textMapping(
    val `raw`: String,
    val `recognized`: String,
    val `replacement`: String
) : BusinessCommandCommand12 { override val `kind`: String = "textMapping" }
