package com.govorun.lite.stylization

import android.content.Context
import android.content.SharedPreferences
import com.govorun.lite.util.StorageKeys

internal enum class StylizationServerMode {
    GOVORUN,
    SELF_HOSTED,
}

internal data class SelfHostedServerSettings(
    val endpoint: String = "",
    val model: String = "",
    val apiKey: String = "",
    val generalPrompt: String = DEFAULT_GENERAL_PROMPT,
    val acceptedRecipient: String = "",
    val proxyEnabled: Boolean = false,
    val proxyURL: String = "",
) {
    val recipient: String? get() = SelfHostedConsent.recipient(endpoint, proxyEnabled, proxyURL)
    val hasConsent: Boolean get() = decision().getBoolean("consent")
    val normalizedEndpoint: String? get() = SelfHostedEndpointPolicy.normalize(endpoint)
    val isComplete: Boolean get() = decision().getBoolean("complete")
    fun decision(previous: SelfHostedServerSettings? = null) = selfHostedDecision(org.json.JSONObject().put("kind", "settings")
        .put("value", wire()).put("previous", previous?.wire() ?: org.json.JSONObject.NULL))
    private fun wire() = org.json.JSONObject().put("endpoint", normalizedEndpoint.orEmpty()).put("model", model)
        .put("key", apiKey).put("prompt", generalPrompt).put("accepted", acceptedRecipient).put("recipient", recipient)
        .put("proxyEnabled", proxyEnabled).put("proxyURL", SelfHostedProxy.normalized(proxyURL).orEmpty())

    companion object {
        const val MAX_MODEL_LENGTH = 120
        const val MAX_API_KEY_LENGTH = 4_096
        const val DEFAULT_GENERAL_PROMPT =
            "Исправь ошибки распознавания, убери повторы и расставь знаки. " +
                "Не меняй смысл. Верни только готовый текст."
    }
}

internal object StylizationServerPreferences {
    private const val KEY_MODE = "stylization_server_mode_v2"
    private const val KEY_ENDPOINT = "stylization_self_hosted_endpoint_v2"
    private const val KEY_MODEL = "stylization_self_hosted_model_v2"
    private const val KEY_PROMPT = "stylization_self_hosted_prompt_v2"
    private const val OLD_API_KEY = "stylization_self_hosted_api_key_v2"
    private const val KEY_API_KEY = "stylization_self_hosted_api_key_v3"
    private const val KEY_STYLIZATION_ENABLED = "stylization_self_hosted_enabled_v2"
    private const val KEY_SELECTED_STYLE = "stylization_self_hosted_style_v2"
    private const val KEY_VERIFIED_CONFIGURATION = "stylization_self_hosted_verified_v2"
    private const val KEY_ACCEPTED_RECIPIENT = "stylization_self_hosted_consent_v1"
    private const val KEY_PROXY_ENABLED = "stylization_self_hosted_proxy_enabled_v1"
    private const val KEY_PROXY_URL = "stylization_self_hosted_proxy_url_v1"
    private const val MODE_SELF_HOSTED = "self_hosted"
    const val GENERAL_STYLE = "general"

    private fun preferences(context: Context) = context.getSharedPreferences(
        StorageKeys.PREFS_FILE,
        Context.MODE_PRIVATE,
    )

    fun mode(context: Context): StylizationServerMode =
        if (preferences(context).getString(KEY_MODE, null) == MODE_SELF_HOSTED) {
            StylizationServerMode.SELF_HOSTED
        } else {
            StylizationServerMode.GOVORUN
        }

    fun setMode(context: Context, mode: StylizationServerMode) {
        preferences(context).edit()
            .putString(KEY_MODE, if (mode == StylizationServerMode.SELF_HOSTED) {
                MODE_SELF_HOSTED
            } else {
                "govorun"
            })
            .apply()
    }

    fun settings(context: Context): SelfHostedServerSettings {
        val prefs = preferences(context)
        val selected = prefs.getString(KEY_SELECTED_STYLE, GENERAL_STYLE).orEmpty()
        val selection = selection(context, selected)
        if (selection.getBoolean("migration")) {
            selection.optString("migrationPrompt").takeUnless { selection.isNull("migrationPrompt") }
                ?.let { prefs.edit().putString(KEY_PROMPT, it).apply() }
            prefs.edit().putString(KEY_SELECTED_STYLE, GENERAL_STYLE).apply()
        }
        return SelfHostedServerSettings(
            endpoint = prefs.getString(KEY_ENDPOINT, "").orEmpty(),
            model = prefs.getString(KEY_MODEL, "").orEmpty(),
            apiKey = readApiKey(prefs, ::readPreviousApiKey).orEmpty(),
            generalPrompt = prefs.getString(
                KEY_PROMPT,
                SelfHostedServerSettings.DEFAULT_GENERAL_PROMPT,
            ).orEmpty(),
            acceptedRecipient = prefs.getString(KEY_ACCEPTED_RECIPIENT, "").orEmpty(),
            proxyEnabled = prefs.getBoolean(KEY_PROXY_ENABLED, false),
            proxyURL = prefs.getString(KEY_PROXY_URL, "").orEmpty(),
        )
    }

    fun saveSettings(context: Context, value: SelfHostedServerSettings): Boolean {
        val previous = settings(context)
        val decision = value.decision(previous)
        if (!decision.getBoolean("complete")) return false
        val endpoint = value.normalizedEndpoint ?: return false
        val model = decision.getString("model")
        val prompt = decision.getString("prompt")
        val apiKey = decision.getString("key")
        val stored = apiKey
        preferences(context).edit()
            .putString(KEY_ENDPOINT, endpoint)
            .putString(KEY_MODEL, model)
            .putString(KEY_PROMPT, prompt)
            .putBoolean(KEY_PROXY_ENABLED, value.proxyEnabled)
            .putString(KEY_PROXY_URL, value.proxyURL.trim())
            .putString(KEY_ACCEPTED_RECIPIENT, decision.getString("accepted"))
            .apply {
                if (decision.getBoolean("invalidate")) remove(KEY_VERIFIED_CONFIGURATION)
                putString(KEY_API_KEY, stored)
                remove(OLD_API_KEY)
            }
            .apply()
        return true
    }

    fun deleteApiKey(context: Context): Boolean = preferences(context).edit()
        .remove(KEY_API_KEY).remove(OLD_API_KEY).remove(KEY_VERIFIED_CONFIGURATION)
        .putBoolean(KEY_STYLIZATION_ENABLED, false).commit()

    fun clearServer(context: Context): Boolean = clearServer(preferences(context))

    internal fun clearServer(prefs: SharedPreferences): Boolean = prefs.edit().apply {
        listOf(KEY_ENDPOINT, KEY_MODEL, KEY_PROMPT, KEY_API_KEY, OLD_API_KEY,
            KEY_SELECTED_STYLE, KEY_VERIFIED_CONFIGURATION, KEY_ACCEPTED_RECIPIENT,
            KEY_PROXY_ENABLED, KEY_PROXY_URL, SelfHostedStylePresets.KEY_PRESETS).forEach { remove(it) }
        putBoolean(KEY_STYLIZATION_ENABLED, false)
        // Keep the selected route disabled until the user explicitly chooses another route.
    }.commit()

    fun revokeConsent(context: Context) {
        preferences(context).edit().remove(KEY_ACCEPTED_RECIPIENT)
            .putBoolean(KEY_STYLIZATION_ENABLED, false).apply()
    }

    fun stylizationEnabled(context: Context): Boolean =
        preferences(context).getBoolean(KEY_STYLIZATION_ENABLED, false)

    fun setStylizationEnabled(context: Context, enabled: Boolean) {
        preferences(context).edit().putBoolean(KEY_STYLIZATION_ENABLED, enabled).apply()
    }

    fun selectedStyle(context: Context): String = selection(context).getString("selected")
    private fun selection(context: Context, selected: String? = preferences(context).getString(KEY_SELECTED_STYLE, GENERAL_STYLE), id: String? = null) =
        selfHostedDecision(org.json.JSONObject().put("kind", "selection").put("selected", selected)
            .put("items", CustomStyleBinding.encode(SelfHostedStylePresets.list(context)))
            .put("prompt", preferences(context).getString(KEY_PROMPT, SelfHostedServerSettings.DEFAULT_GENERAL_PROMPT)).put("id", id))

    fun selectGeneral(context: Context) {
        preferences(context).edit().putString(KEY_SELECTED_STYLE, GENERAL_STYLE).apply()
    }

    fun selectCustom(context: Context, id: String): Boolean {
        if (!selection(context, id = id).getBoolean("canSelect")) return false
        preferences(context).edit().putString(KEY_SELECTED_STYLE, "custom:$id").apply()
        return true
    }

    fun resolvedInstruction(context: Context): String? {
        val value = selection(context)
        return value.optString("instruction").takeUnless { value.isNull("instruction") }
    }

    fun markVerified(context: Context): Boolean {
        val value = settings(context)
        val fingerprint = SelfHostedEndpointPolicy.fingerprint(value.endpoint, value.model)
            ?: return false
        preferences(context).edit().putString(KEY_VERIFIED_CONFIGURATION, fingerprint).apply()
        return true
    }

    fun isVerified(context: Context): Boolean {
        val value = settings(context)
        val fingerprint = SelfHostedEndpointPolicy.fingerprint(value.endpoint, value.model)
            ?: return false
        return preferences(context).getString(KEY_VERIFIED_CONFIGURATION, null) == fingerprint
    }

    internal fun readApiKey(prefs: SharedPreferences, decrypt: (String) -> String?): String? {
        prefs.getString(KEY_API_KEY, null)?.let { return it }
        val previous = prefs.getString(OLD_API_KEY, null) ?: return null
        val key = decrypt(previous) ?: return null
        // Leave the original intact unless the replacement is durably saved.
        prefs.edit().putString(KEY_API_KEY, key).commit()
        return key
    }

    private fun readPreviousApiKey(raw: String): String? = runCatching {
        val store = java.security.KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val key = store.getKey("govorun_self_hosted_api_key_v2", null) as? javax.crypto.SecretKey
            ?: return null
        val joined = android.util.Base64.decode(raw, android.util.Base64.NO_WRAP)
        require(joined.size > 12)
        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(javax.crypto.Cipher.DECRYPT_MODE, key,
            javax.crypto.spec.GCMParameterSpec(128, joined.copyOfRange(0, 12)))
        String(cipher.doFinal(joined.copyOfRange(12, joined.size)), Charsets.UTF_8)
    }.getOrNull()

}
