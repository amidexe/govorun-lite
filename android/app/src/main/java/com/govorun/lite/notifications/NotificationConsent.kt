package com.govorun.lite.notifications

import com.govorun.lite.account.AccountState
import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal fun notificationCommand(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
    .put("operation", "optionalNotifications").put("command", command)).getJSONObject("value")
internal fun notificationAccountScope(account: AccountState, origin: String): String = notificationCommand(JSONObject()
    .put("kind", "scope").put("available", account.available).put("signedIn", account.signedIn)
    .put("storageProblem", account.secureStorageProblem != null).put("accountId", account.accountId).put("origin", origin)).getString("scope")
internal data class NotificationConsentRequest(val accountScope: String, val revision: Long) {
    fun wire() = JSONObject().put("accountScope", accountScope).put("revision", revision)
}
internal fun NotificationState.withFirstSeenAt(now: Long) = changed("firstSeen", now = now)!!
internal fun NotificationState.withAccount(currentScope: String, permission: Boolean, now: Long = 0) =
    changed("account", currentScope, permission, now)!!
internal fun NotificationState.shouldInvite(supported: Boolean, currentScope: String, now: Long) =
    decision("invite", currentScope, now = now, extra = JSONObject().put("supported", supported)).getBoolean("allowed")
internal fun NotificationState.canAccept(request: NotificationConsentRequest, currentScope: String) =
    decision("canAccept", currentScope, extra = JSONObject().put("request", request.wire())).getBoolean("allowed")
internal fun NotificationState.mayDeliver(currentScope: String, permission: Boolean) = decision("deliver", currentScope, permission).getBoolean("allowed")
