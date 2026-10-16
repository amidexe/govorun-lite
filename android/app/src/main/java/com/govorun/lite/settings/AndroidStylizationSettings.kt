package com.govorun.lite.settings

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.settings.StylizationPreferenceEffects
import com.govorun.lite.settings.StylizationPreferences
import com.govorun.lite.settings.StylizationSettingsController
import com.govorun.lite.util.Prefs

/** Composition for preference writes that affect official access. */
internal object AndroidStylizationSettings {
    fun create(context: Context): StylizationSettingsController {
        val app = context.applicationContext
        return StylizationSettingsController(
            preferences = object : StylizationPreferences {
                override fun setEnabled(enabled: Boolean) = Prefs.setStylizationEnabled(app, enabled)
                override fun dismissTermsUpdate() = Prefs.dismissStylizationTermsUpdate(app)
                override fun acceptTerms(version: Int, acceptedAtMillis: Long) =
                    Prefs.acceptStylizationTerms(app, version, acceptedAtMillis)
            },
            effects = object : StylizationPreferenceEffects {
                override fun preferenceChanged() {
                    GovorunAccountRepository.onStylizationPreferenceChanged()
                }
                override fun consentScopeChanged() {
                    GovorunAccountRepository.onStylizationConsentScopeChanged()
                }
            },
        )
    }
}
