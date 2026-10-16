package com.govorun.lite.account

import org.json.JSONObject

/** Raw I/O requested by the common owner; it supplies every authorization header. */
internal interface AccountHttpPort {
    val configured: Boolean
    suspend fun send(
        path: String,
        method: String,
        body: JSONObject? = null,
        accessToken: String? = null,
        headers: Map<String, String> = emptyMap(),
    ): AccountHttpResponse
}
