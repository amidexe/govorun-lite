package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusinessException
import org.json.JSONObject
import org.json.JSONTokener

/** Private I/O edge of the persistent shared session. Only the Rust owner may
 * issue these effects; callers submit account commands through its protocol.
 */
internal class SharedAccountEffects(
    private val storage: SharedAccountStorage,
    private val transport: AccountHttpPort,
    private val project: (JSONObject) -> Unit,
    private val extraEffect: suspend (JSONObject) -> JSONObject = { throw SharedBusinessException("account_invalid_request", 0) },
) {
    suspend fun execute(command: JSONObject): JSONObject = when (command.getString("operation")) {
        "storage" -> storage.execute(command)
        "project" -> { project(command.getJSONObject("value")); JSONObject() }
        "http" -> http(command)
        else -> extraEffect(command)
    }

    private suspend fun http(command: JSONObject): JSONObject {
        val method = command.getString("method")
        val headers = buildMap {
            command.optString("etag").takeIf { !command.isNull("etag") && it.isNotEmpty() }?.let {
                put(if (method == "GET") "If-None-Match" else "If-Match", it)
            }
            command.optString("idempotencyKey").takeIf { !command.isNull("idempotencyKey") && it.isNotEmpty() }
                ?.let { put("Idempotency-Key", it) }
        }
        val response = try {
            transport.send(command.getString("path"), method, command.optJSONObject("body"),
                command.optString("accessToken").takeUnless { command.isNull("accessToken") }, headers)
        } catch (cancelled: kotlinx.coroutines.CancellationException) { throw cancelled }
        catch (_: Exception) { throw SharedBusinessException("account_network_unavailable", 0) }
        val body = if (response.body.isBlank()) JSONObject.NULL else try {
            JSONTokener(response.body).nextValue()
        } catch (_: Exception) {
            if (response.statusCode in 200..299) throw SharedBusinessException("account_invalid_response", 0)
            JSONObject.NULL
        }
        return JSONObject().put("status", response.statusCode).put("body", body)
            .put("etag", response.etag ?: JSONObject.NULL)
            .put("retryAfterSeconds", response.retryAfterMillis?.let { (it + 999) / 1_000 } ?: JSONObject.NULL)
    }
}
