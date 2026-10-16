package com.govorun.lite.stylization

import java.net.URI
import org.json.JSONObject
import com.govorun.lite.shared.SharedBusiness

internal fun selfHostedDecision(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
    .put("operation", "androidSelfHostedSettings").put("command", command))

internal fun selfHostedAddress(value: String, proxy: Boolean): URI {
    val uri = URI(if (proxy) value.trim() else value.trim().trimEnd('/')).normalize()
    val parsed = JSONObject().put("scheme", uri.scheme).put("host", uri.host).put("port", uri.port)
        .put("path", uri.path.orEmpty()).put("query", uri.query).put("fragment", uri.fragment).put("userInfo", uri.userInfo)
    val result = selfHostedDecision(JSONObject().put("kind", "address").put("proxy", proxy).put("parsed", parsed))
    return URI(result.getString("scheme"), null, result.getString("host"), result.getInt("port"),
        result.getString("path").takeIf { it.isNotEmpty() }, uri.query, null)
}
internal object SelfHostedEndpointPolicy {
    fun normalize(value: String): String? = runCatching { selfHostedAddress(value, false).toASCIIString() }.getOrNull()
    fun fingerprint(endpoint: String, model: String): String? = normalize(endpoint)?.let {
        selfHostedDecision(JSONObject().put("kind", "fingerprint").put("endpoint", it).put("model", model))
            .optString("value").takeUnless { value -> value == "null" || value.isEmpty() }
    }
}
