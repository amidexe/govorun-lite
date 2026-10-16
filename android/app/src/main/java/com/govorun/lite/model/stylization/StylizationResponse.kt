package com.govorun.lite.model.stylization

data class StylizationResponse(
    val text: String,
    val outcomePass: String? = null,
    val latencyBucket: String = "unknown",
)

