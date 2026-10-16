package com.govorun.lite.account

import kotlinx.coroutines.flow.MutableStateFlow
import org.json.JSONObject

/** Display only. Epoch tags let styles arrive before the matching account view. */
internal class SharedStylesProjection(
    private val transitions: AccountSessionTransitionGate,
    private val state: MutableStateFlow<AccountState>,
) {
    private var epoch = -1L
    private var value: JSONObject? = null
    fun project(command: JSONObject) = transitions.locked {
        val incoming = command.getLong("sessionGeneration")
        if (incoming < epoch) return@locked
        epoch = incoming
        value = command.getJSONObject("value")
        applyCurrent()
    }
    fun applyCurrent() = transitions.locked {
        state.value = decorate(state.value)
    }
    fun decorate(current: AccountState): AccountState {
        if (epoch != current.sessionGeneration) return current
        val view = value ?: return current
        return current.copy(styleWindowJson = view.optJSONObject("window")?.toString(),
            secureStorageProblem = if (view.optString("error") in setOf(
                "account_storage_unavailable", "account_storage_invalid"))
                SecureStorageProblem.TEMPORARILY_UNAVAILABLE else current.secureStorageProblem)
    }
}
