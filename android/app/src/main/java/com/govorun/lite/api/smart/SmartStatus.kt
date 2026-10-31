package com.govorun.lite.api.smart

import com.govorun.lite.network.NetworkRefusal
import org.json.JSONObject

/** Everything getStatus looks at; all of it is already in memory. */
internal data class SmartFacts(
    val authorized: Boolean,
    val signedIn: Boolean = false,
    val refusal: NetworkRefusal? = null,
    val networkAvailable: Boolean = false,
)

internal data class SmartStatus(val state: SmartState) {
    /** An unauthorized caller learns nothing about the account. */
    fun json(): String = JSONObject().put("state", state.wire).put("message", state.message).toString()
}

/**
 * The first reason Echo may not ask now: permission, account, the person's network choice, the network. Access, limits and
 * consent are Govorun's own text improvement's to check, on each improvement.
 */
internal fun smartStatus(facts: SmartFacts): SmartStatus = SmartStatus(
    when {
        !facts.authorized -> SmartState.NOT_AUTHORIZED
        !facts.signedIn -> SmartState.NOT_SIGNED_IN
        facts.refusal == NetworkRefusal.OFFLINE -> SmartState.OFFLINE
        facts.refusal == NetworkRefusal.ROAMING -> SmartState.ROAMING
        !facts.networkAvailable -> SmartState.NO_NETWORK
        else -> SmartState.READY
    },
)
