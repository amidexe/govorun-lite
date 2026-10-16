package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.util.StorageKeys
import org.json.JSONObject
import com.govorun.lite.shared.SharedBusiness

/** Current preference keys and DTO binding; Core selects and resolves options. */
internal object StylizationStylePreferences {
    private const val KEY_OPTIONS = "stylization_style_options_v1"
    private const val KEY_APP_PROFILES = "stylization_app_style_profiles_v2"
    fun selectedValue(context: Context): String = call(context, JSONObject().put("kind", "snapshot")).getString("selected")
    fun optionsForSelected(context: Context): StyleOptions = options(context, selectedValue(context))
    fun options(context: Context, value: String): StyleOptions = styleOptions(call(context,
        JSONObject().put("kind", "options").put("value", value)).getJSONObject("options"))
    fun setSelectedOptions(context: Context, options: StyleOptions) {
        val value = selectedValue(context)
        // Catalog updates still execute through the shared library/store transaction.
        if (value.startsWith("custom:")) { AndroidCustomStyles.create(context).updateOptions(value.removePrefix("custom:"), options); return }
        val root = optionObject(context)
        root.put(value, CustomStyleBinding.options(options).put("emojiEnabled", options.emojiEnabled))
        preferences(context).edit().putString(KEY_OPTIONS, root.toString()).apply()
    }
    fun appProfiles(context: Context): Map<String, ApplicationStyleProfile> = runCatching {
        val values = profileDecision(JSONObject().put("kind", "profiles").put("profiles", profileObject(context))).getJSONObject("profiles")
        values.keys().asSequence().associateWith { key -> ApplicationStyleProfile.read(values.getJSONObject(key)) }
    }.getOrDefault(emptyMap())
    fun setAppProfile(context: Context, packageName: String, profile: ApplicationStyleProfile?) {
        val result = profileDecision(JSONObject().put("kind", "update").put("profiles", profileObject(context))
            .put("package", packageName).put("profile", profile?.wire() ?: JSONObject.NULL))
        preferences(context).edit().putString(KEY_APP_PROFILES, result.getJSONObject("profiles").toString()).apply()
    }
    fun resolve(context: Context, packageName: String?): ResolvedStylizationStyle {
        val result = call(context, JSONObject().put("kind", "resolve").put("package", packageName)
            .put("profiles", profileObject(context)))
        val payload = result.getJSONObject("options")
        return ResolvedStylizationStyle(result.getBoolean("enabled"), TextStyle.fromApiId(payload.getString("style")),
            result.optJSONObject("preset")?.let(CustomStyleBinding::preset), styleOptions(payload))
    }
    private fun call(context: Context, command: JSONObject): JSONObject = profileDecision(command
        .put("state", AndroidCustomStyleStorage(context).read()).put("options", optionObject(context)))
    private fun optionObject(context: Context) = runCatching { JSONObject(preferences(context).getString(KEY_OPTIONS, "{}")) }.getOrDefault(JSONObject())
    private fun profileObject(context: Context) = runCatching { JSONObject(preferences(context).getString(KEY_APP_PROFILES, "{}")) }.getOrDefault(JSONObject())
    private fun preferences(context: Context) = context.getSharedPreferences(StorageKeys.PREFS_FILE, Context.MODE_PRIVATE)
}
internal fun profileDecision(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
    .put("operation", "androidStylePreferences").put("command", command))
internal fun styleOptions(item: JSONObject): StyleOptions = StyleOptions(
    PunctuationMode.fromApiId(item.getString("punctuationMode")), item.getString("emojiMode") != "off",
    item.getBoolean("profanityFilterEnabled"), EmojiMode.fromApiId(item.getString("emojiMode")))
internal data class ApplicationStyleProfile(val styleValue: String? = null, val emojiEnabled: Boolean? = null,
    val reducedPunctuationEnabled: Boolean? = null, val emojiMode: EmojiMode? = null) {
    fun isInherited(): Boolean = decision(StyleOptions()).getBoolean("inherited")
    fun hasValidStyle(): Boolean = decision(StyleOptions()).getBoolean("valid")
    fun applyTo(base: StyleOptions): StyleOptions = styleOptions(decision(base).getJSONObject("options"))
    private fun decision(base: StyleOptions) = profileDecision(JSONObject().put("kind", "profile")
        .put("profile", wire()).put("base", CustomStyleBinding.options(base)))
    internal fun wire() = JSONObject().put("styleValue", styleValue).put("emojiEnabled", emojiEnabled)
        .put("emojiMode", emojiMode?.apiId).put("reducedPunctuationEnabled", reducedPunctuationEnabled)
    companion object {
        fun read(item: JSONObject) = ApplicationStyleProfile(item.optString("styleValue").takeUnless { !item.has("styleValue") },
            item.opt("emojiEnabled") as? Boolean, item.opt("reducedPunctuationEnabled") as? Boolean,
            item.optString("emojiMode").takeUnless { !item.has("emojiMode") }?.let { EmojiMode.fromApiId(it) })
    }
}
