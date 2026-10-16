package com.govorun.lite.account

import org.json.JSONObject

internal class AccountApiClient(
    private val requests: AccountRequestPort,
    val configured: Boolean,
) {
    suspend fun readPreferenceDocument(accessToken: String): JSONObject =
        documentRequest(accessToken, JSONObject().put("operation", "syncPreferencesRead"))

    suspend fun writePreferenceDocument(accessToken: String, patch: JSONObject): JSONObject =
        documentRequest(accessToken, JSONObject().put("operation", "syncPreferencesPatch").put("patch", patch))

    suspend fun syncDocument(accessToken: String, etag: String?): SyncDocumentResponse {
        val response = documentResponse(accessToken, JSONObject().put("operation", "syncContentRead").put("etag", etag))
        return SyncDocumentJson.decodeResponse(response.body, response.etag, response.statusCode, etag.orEmpty())
    }

    suspend fun putSyncDocument(
        accessToken: String,
        etag: String,
        document: SyncedContentDocument,
    ): SyncDocumentResponse {
        val response = documentResponse(accessToken, JSONObject().put("operation", "syncContentWrite")
            .put("etag", etag).put("document", SyncDocumentJson.encode(document)))
        return SyncDocumentJson.decodeResponse(response.body, requireNotNull(response.etag), response.statusCode)
    }

    suspend fun deleteAccount(accessToken: String) {
        documentResponse(accessToken, JSONObject().put("operation", "accountDelete").put("confirmed", true))
    }

    internal suspend fun requestJson(
        path: String,
        method: String,
        body: JSONObject? = null,
        accessToken: String? = null,
        headers: Map<String, String> = emptyMap(),
    ): JSONObject = JSONObject(
        request(path, method, body, accessToken, headers).body,
    )

    internal suspend fun request(
        path: String,
        method: String,
        body: JSONObject? = null,
        accessToken: String? = null,
        headers: Map<String, String> = emptyMap(),
        acceptedStatuses: Set<Int> = emptySet(),
    ): AccountHttpResponse {
        val response = requests.send(path, method, body, accessToken, headers, false)
        if (response.statusCode !in 200..299 && response.statusCode !in acceptedStatuses) {
            throw AccountHttpException(response.statusCode, response.body, response.retryAfterMillis)
        }
        return response
    }

    internal suspend fun documentRequest(token: String, command: JSONObject): JSONObject =
        JSONObject(documentResponse(token, command).body)

    internal suspend fun documentResponse(token: String, command: JSONObject): AccountHttpResponse {
        val wire = com.govorun.lite.shared.SharedBusiness.evaluate(JSONObject()
            .put("operation", "accountDocumentRequest").put("command", command))
        val headers = wire.getJSONObject("headers").let { value -> value.keys().asSequence().associateWith(value::getString) }
        return request(wire.getString("path"), wire.getString("method"), wire.optJSONObject("body"), token, headers,
            wire.getJSONArray("acceptedStatuses").let { list -> (0 until list.length()).map(list::getInt).toSet() })
    }


}

internal const val USER_AGREEMENT_VERSION = "1.0"
