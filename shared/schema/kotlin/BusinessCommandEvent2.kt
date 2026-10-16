// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent2 { val `kind`: String }
data class BusinessCommandEvent2_styleProcessing(
    val `account`: String,
    val `failed`: Boolean,
    val `slow`: Boolean
) : BusinessCommandEvent2 { override val `kind`: String = "styleProcessing" }
data class BusinessCommandEvent2_words(
    val `count`: Long
) : BusinessCommandEvent2 { override val `kind`: String = "words" }
data class BusinessCommandEvent2_seconds(
    val `seconds`: Long
) : BusinessCommandEvent2 { override val `kind`: String = "seconds" }
data class BusinessCommandEvent2_performance(
    val `processing_milliseconds`: Long,
    val `speech_milliseconds`: Long,
    val `success`: Boolean
) : BusinessCommandEvent2 { override val `kind`: String = "performance" }
data class BusinessCommandEvent2_session(
    val `speech_seconds`: Long,
    val `text`: String
) : BusinessCommandEvent2 { override val `kind`: String = "session" }
data class BusinessCommandEvent2_sessionFacts(
    val `speech_seconds`: Long,
    val `words`: Long
) : BusinessCommandEvent2 { override val `kind`: String = "sessionFacts" }
data class BusinessCommandEvent2_recognitionFacts(
    val `processing_milliseconds`: Long,
    val `speech_milliseconds`: Long,
    val `success`: Boolean,
    val `words`: Long
) : BusinessCommandEvent2 { override val `kind`: String = "recognitionFacts" }
data class BusinessCommandEvent2_recognition(
    val `processing_milliseconds`: Long,
    val `speech_milliseconds`: Long,
    val `success`: Boolean,
    val `text`: String
) : BusinessCommandEvent2 { override val `kind`: String = "recognition" }
