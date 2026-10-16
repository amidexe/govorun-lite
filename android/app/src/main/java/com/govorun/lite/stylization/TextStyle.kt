package com.govorun.lite.stylization

import androidx.annotation.StringRes
import com.govorun.lite.R
import com.govorun.lite.model.stylization.TextStyle

@get:StringRes
val TextStyle.titleRes: Int get() = when (this) {
    TextStyle.FORMAL -> R.string.stylization_style_formal
    TextStyle.LIGHT -> R.string.stylization_style_light
    TextStyle.CONCISE -> R.string.stylization_style_concise
    TextStyle.CUSTOM -> R.string.stylization_style_custom
}

@get:StringRes
val TextStyle.descriptionRes: Int get() = when (this) {
    TextStyle.FORMAL -> R.string.stylization_style_formal_body
    TextStyle.LIGHT -> R.string.stylization_style_light_body
    TextStyle.CONCISE -> R.string.stylization_style_concise_body
    TextStyle.CUSTOM -> R.string.stylization_style_custom_body
}
