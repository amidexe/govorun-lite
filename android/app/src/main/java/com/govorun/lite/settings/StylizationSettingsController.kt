package com.govorun.lite.settings

import org.json.JSONObject

internal interface StylizationPreferences {
    fun setEnabled(enabled: Boolean)
    fun dismissTermsUpdate()
    fun acceptTerms(version: Int, acceptedAtMillis: Long)
}

internal interface StylizationPreferenceEffects {
    fun preferenceChanged()
    fun consentScopeChanged()
}

/** Publishes changes only after the local preference write succeeds. */
internal class StylizationSettingsController(
    private val preferences: StylizationPreferences,
    private val effects: StylizationPreferenceEffects,
) {
    fun setEnabled(enabled: Boolean) = execute(JSONObject().put("kind", "enable").put("enabled", enabled))
    fun dismissTermsUpdate() = execute(JSONObject().put("kind", "dismissTerms"))
    fun acceptTerms(version: Int, acceptedAtMillis: Long) = execute(JSONObject()
        .put("kind", "acceptTerms").put("version", version).put("acceptedAtMillis", acceptedAtMillis))

    private fun execute(command: JSONObject) {
        SettingsRules.effects(SettingsRules.evaluate(command)).forEach { effect ->
            when (effect.getString("kind")) {
                "setEnabled" -> preferences.setEnabled(effect.getBoolean("enabled"))
                "dismissTerms" -> preferences.dismissTermsUpdate()
                "acceptTerms" -> preferences.acceptTerms(effect.getInt("version"), effect.getLong("acceptedAtMillis"))
                "preferenceChanged" -> effects.preferenceChanged()
                "consentScopeChanged" -> effects.consentScopeChanged()
                else -> error("Invalid stylization settings effect")
            }
        }
    }
}
