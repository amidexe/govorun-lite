package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.model.stylization.TextStyle

internal data class ResolvedStylizationStyle(
    val enabled: Boolean,
    val style: TextStyle,
    val customPreset: CustomStylePreset? = null,
    val options: StyleOptions = StyleOptions(),
)
