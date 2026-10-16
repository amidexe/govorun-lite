package com.govorun.lite.account

import org.json.JSONObject

/** The screen holds a session reference, never a second account or checkout owner. */
internal suspend fun BillingEnvironment.command(operation: BillingOperation, value: JSONObject): JSONObject {
    operation.ensureCurrent()
    val result = accountCommand(value.put("sessionGeneration", operation.scope.generation))
    operation.ensureCurrent()
    return result
}

internal fun JSONObject.accountResponseBody(): JSONObject {
    val status = getInt("status")
    val body = optJSONObject("body") ?: JSONObject()
    if (status !in 200..299) throw AccountHttpException(status, body.toString())
    return body
}
