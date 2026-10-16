package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.shared.SharedBusinessException
import com.govorun.lite.stylization.NetworkAvailability
import com.govorun.lite.stylization.StylizationServerMode
import com.govorun.lite.stylization.StylizationServerPreferences
import com.govorun.lite.util.Prefs
import org.json.JSONObject

/** Physical Android effects only. Core owns all timing, retries and custody. */
internal class SharedStylesEffects(
    context: Context,
    private val project: (JSONObject) -> Unit,
    private val availability: (Boolean) -> Unit,
    private val progress: (Long, JSONObject) -> Unit,
) {
    private val app = context.applicationContext
    private val vault = SharedStyleVaultStorage(app)

    suspend fun execute(command: JSONObject): JSONObject = when (command.getString("operation")) {
        "styleVault" -> vault.execute(command)
        "styleFacts" -> JSONObject().put("enabled",Prefs.isStylizationEnabled(app))
            .put("selfHosted",StylizationServerPreferences.mode(app) == StylizationServerMode.SELF_HOSTED)
            .put("acceptedTerms",Prefs.getStylizationConsentVersion(app))
            .put("networkAvailable",NetworkAvailability.hasUsableNetwork(app))
        "stylesProject" -> { project(command); JSONObject() }
        "stylesAvailability" -> { availability(command.getBoolean("value")); JSONObject() }
        "styleProgress" -> { progress(command.getLong("callId"),command.getJSONObject("value")); JSONObject() }
        else -> throw SharedBusinessException("account_invalid_request",0)
    }

}
