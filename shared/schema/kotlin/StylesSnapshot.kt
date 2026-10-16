// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class StylesSnapshot(
    val `applicationRules`: List<ApplicationRule>,
    val `applications`: List<ApplicationChoice>,
    val `availability`: String?,
    val `availabilityReason`: String,
    val `busy`: Boolean,
    val `canCreatePreset`: Boolean,
    val `displayState`: String,
    val `enabled`: Boolean,
    val `gate`: String,
    val `limitReason`: String,
    val `maxPresets`: Long,
    val `message`: String?,
    val `onlineServicePaused`: Boolean,
    val `options`: TextStyleOptions,
    val `presets`: List<StylePreset>,
    val `processingLimits`: StylesProcessingLimits,
    val `processingReadiness`: String,
    val `resumeChoice`: Boolean,
    val `scope`: String,
    val `selected`: String?,
    val `styles`: List<StyleChoice>,
    val `termsAccepted`: Boolean,
    val `termsVersion`: Int,
    val `title`: String
)
