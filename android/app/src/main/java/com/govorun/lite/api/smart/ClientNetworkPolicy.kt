package com.govorun.lite.api.smart

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import org.json.JSONObject

/** Local Binder choices for the caller's jobs; never changes Govorun's own network switches. */
internal class ClientNetworkPolicy(context: Context, private val changed: () -> Unit) {
    private val app = context.applicationContext
    private val lock = Any()
    private data class Choice(val offline: Boolean, val noRoaming: Boolean)
    // Echo owns the settings. After a process restart, wait for its existing receiver to confirm them.
    private val choices = mutableMapOf<String, Choice>()
    private val manager = app.getSystemService(ConnectivityManager::class.java)
    @Volatile private var network = manager.activeNetwork
    @Volatile private var caps = network?.let(manager::getNetworkCapabilities)

    init {
        manager.registerDefaultNetworkCallback(object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(value: Network) { network = value; caps = null; changed() }
            override fun onCapabilitiesChanged(value: Network, capabilities: NetworkCapabilities) {
                if (network == value) { caps = capabilities; changed() }
            }
            override fun onLost(value: Network) { if (network == value) { network = null; caps = null; changed() } }
        })
    }

    fun set(owner: String, json: String) {
        val value = JSONObject(json)
        require(value.keys().asSequence().toSet() == setOf("offline", "noRoaming"))
        require(value.get("offline") is Boolean && value.get("noRoaming") is Boolean)
        val next = Choice(value.getBoolean("offline"), value.getBoolean("noRoaming"))
        synchronized(lock) {
            if (choices[owner] == next) return
            choices[owner] = next
        }
        changed()
    }

    fun refusal(owner: String): String? = synchronized(lock) {
        val value = choices[owner] ?: return@synchronized OFFLINE
        reason(value.offline, value.noRoaming,
            caps != null, caps?.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_ROAMING) == true)
    }

    companion object {
        const val CAPABILITY = "clientNetworkPolicy"
        const val KIND = "client_network_policy"
        const val OFFLINE = "echo_offline"
        const val ROAMING = "echo_roaming"
        fun reason(offline: Boolean, noRoaming: Boolean, connected: Boolean, notRoaming: Boolean): String? = when {
            offline -> OFFLINE
            !connected -> SmartState.NO_NETWORK.wire
            noRoaming && !notRoaming -> ROAMING
            else -> null
        }
    }
}
