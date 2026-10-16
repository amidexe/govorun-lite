package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

/** Local common decisions; this adapter never owns enrollment, ordering or retry rules. */
internal object SynchronizationRules {
    fun evaluate(command: JSONObject): JSONObject = SharedBusiness.evaluate(
        JSONObject().put("operation", "synchronization").put("command", command))
    fun scope(accountId: String) = JSONObject().put("account", accountId).put("generation", 0)
}
