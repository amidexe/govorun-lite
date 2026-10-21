package com.govorun.lite.settings

import com.govorun.lite.model.settings.SettingsChange
import org.json.JSONObject

internal interface SettingsPreferences {
    val stylizationEnabled: Boolean
    fun disableStylization()
    fun persist(change: SettingsChange)
    fun reset(defaults: JSONObject)
    fun resetAppearance()
}

internal enum class SettingsEffect { THEME, SIZE, ALPHA, SIDE, EDGE_MARGIN }

internal fun interface SettingsRuntime {
    fun apply(effects: Set<SettingsEffect>)
}

internal data class SettingsChangeResult(val stylizationDisabled: Boolean = false)

internal interface SettingsAccountEffects {
    fun stylizationDisabled()
    fun beforeReset()
    fun afterReset(defaults: JSONObject)
}

/** Applies preference rules before notifying the running platform service. */
internal class SettingsController(
    private val preferences: SettingsPreferences,
    private val runtime: SettingsRuntime,
    private val account: SettingsAccountEffects,
) {
    fun apply(change: SettingsChange): SettingsChangeResult {
        var disabled = false
        if (change is SettingsChange.SetRecordingMode) {
            val decision = SettingsRules.recording("select", change.value, preferences.stylizationEnabled)
            disabled = decision.getBoolean("stylizationDisabled")
            SettingsRules.effects(decision).forEach { effect ->
                when (effect.getString("kind")) {
                    "disableStylization" -> preferences.disableStylization()
                    "preferenceChanged" -> account.stylizationDisabled()
                    "persistRecording" -> preferences.persist(change)
                    else -> error("Invalid recording settings effect")
                }
            }
        } else preferences.persist(change)
        val effect = when (change) {
            is SettingsChange.SetThemeMode, is SettingsChange.SetDynamicColor -> SettingsEffect.THEME
            is SettingsChange.SetBubbleSize -> SettingsEffect.SIZE
            is SettingsChange.SetBubbleAlpha,
            is SettingsChange.SetBubbleRecordingAlpha,
            is SettingsChange.SetBirdColor,
            -> SettingsEffect.ALPHA
            is SettingsChange.SetBubbleSide -> SettingsEffect.SIDE
            is SettingsChange.SetEdgeMargin -> SettingsEffect.EDGE_MARGIN
            else -> null
        }
        if (effect != null) runtime.apply(setOf(effect))
        return SettingsChangeResult(stylizationDisabled = disabled)
    }

    /** Only the floating bird's look, as listed under «Внешний вид». */
    fun resetAppearance() {
        preferences.resetAppearance()
        runtime.apply(setOf(SettingsEffect.SIZE, SettingsEffect.ALPHA, SettingsEffect.SIDE, SettingsEffect.EDGE_MARGIN))
    }

    fun reset() {
        SettingsRules.effects(SettingsRules.evaluate(JSONObject().put("kind", "reset"))).forEach { effect ->
            when (effect.getString("kind")) {
                "beforeReset" -> account.beforeReset()
                "resetLocal" -> preferences.reset(effect.getJSONObject("defaults"))
                "preferenceChanged" -> account.stylizationDisabled()
                "afterReset" -> account.afterReset(effect.getJSONObject("defaults"))
                "refreshAppearance" -> runtime.apply(SettingsEffect.entries.toSet())
                else -> error("Invalid settings reset effect")
            }
        }
    }
}
