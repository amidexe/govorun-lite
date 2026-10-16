package com.govorun.lite.settings

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.settings.SettingsAccountEffects
import com.govorun.lite.settings.SettingsController
import com.govorun.lite.settings.SettingsEffect
import com.govorun.lite.settings.SettingsRuntime
import org.json.JSONObject
import com.govorun.lite.service.LiteAccessibilityService

/** Composition root for the settings feature. It retains no activity or service. */
internal object AndroidSettingsModule {
    fun create(context: Context) = SettingsController(
        preferences = AndroidSettingsPreferences(context),
        account = object : SettingsAccountEffects {
            private val app = context.applicationContext
            override fun stylizationDisabled() {
                GovorunAccountRepository.onStylizationPreferenceChanged()
            }
            override fun beforeReset() {
                GovorunAccountRepository.recordSyncedContentChange(app)
            }
            override fun afterReset(defaults: JSONObject) {
                GovorunAccountRepository.recordPortablePreferenceChange(
                    app,
                    defaultStyle = defaults.getString("defaultStyle"),
                    reducedPunctuation = false,
                    punctuationMode = defaults.getString("punctuationMode"),
                    shortPhraseThreshold = defaults.getInt("shortPhraseThreshold"),
                )
            }
        },
        runtime = SettingsRuntime { effects ->
            LiteAccessibilityService.instance?.run {
                if (SettingsEffect.THEME in effects) applyBubbleThemeFromPrefs()
                if (SettingsEffect.SIZE in effects) applyBubbleSizeFromPrefs()
                if (SettingsEffect.ALPHA in effects) applyBubbleAlphaFromPrefs()
                if (SettingsEffect.SIDE in effects) applyBubbleSideFromPrefs()
                if (SettingsEffect.EDGE_MARGIN in effects) applyBubbleEdgeMarginFromPrefs()
            }
        },
    )
}
