// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandReport(
    val `days`: List<BusinessCommandDay2>,
    val `requests`: Map<String, BusinessCommandRequestCount>,
    val `scopeId`: String
)
