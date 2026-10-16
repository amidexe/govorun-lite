package com.govorun.lite.account

import org.json.JSONObject

internal suspend fun AccountApiClient.devices(
    accessToken: String,
    currentDeviceName: String? = null,
): Pair<Int, List<AccountDevice>> {
    val command = if (currentDeviceName == null) JSONObject().put("operation", "accountDevices")
        else JSONObject().put("operation", "accountUpdateDevice").put("deviceName", currentDeviceName)
    val json = documentRequest(accessToken, command)
    return json.getInt("deviceLimit") to parseDevices(json.getJSONArray("devices"))
}

internal suspend fun AccountApiClient.revokeDevice(accessToken: String, deviceId: String) {
    documentResponse(accessToken, JSONObject().put("operation", "accountRevokeDevice").put("deviceId", deviceId))
}
