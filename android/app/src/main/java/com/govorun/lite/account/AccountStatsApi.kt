package com.govorun.lite.account

import org.json.JSONObject

internal suspend fun AccountApiClient.statisticsDocument(accessToken: String, request: JSONObject): JSONObject =
    documentRequest(accessToken, JSONObject().put("operation", "syncStatisticsWrite").put("request", request))

internal suspend fun AccountApiClient.deleteStats(accessToken: String) {
    documentResponse(accessToken, JSONObject().put("operation", "syncStatisticsDelete").put("confirmedReset", true))
}
