package com.govorun.lite.account

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities

/** Android connectivity facts, including per-app blocking, plus recovery triggers. */
internal class AccountNetworkRecovery(
    private val context: Context,
    private val onConnectionChanged: (Boolean) -> Unit = {},
    private val onNetworkAvailable: () -> Unit,
) {
    private val manager = context.getSystemService(ConnectivityManager::class.java)
    private val signal = NetworkRecoverySignal()
    private val observation = AccountConnectionObservation()
    private var currentNetwork: Network? = null
    private var capabilities: NetworkCapabilities? = null
    private var blocked = false
    private val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) = available(network)

        override fun onCapabilitiesChanged(network: Network, next: NetworkCapabilities) = synchronized(this@AccountNetworkRecovery) {
            if (network == currentNetwork) {
                capabilities = next
                update()
            }
        }

        override fun onBlockedStatusChanged(network: Network, next: Boolean) = synchronized(this@AccountNetworkRecovery) {
            if (network == currentNetwork) {
                blocked = next
                update()
            }
        }

        override fun onLost(network: Network) = synchronized(this@AccountNetworkRecovery) {
            if (network == currentNetwork) {
                currentNetwork = null
                capabilities = null
                update()
            }
        }

        override fun onUnavailable() = synchronized(this@AccountNetworkRecovery) {
            currentNetwork = null
            capabilities = null
            update()
        }
    }

    fun start() {
        available(manager?.activeNetwork)
        runCatching { manager?.registerDefaultNetworkCallback(callback) }
    }

    @Synchronized
    fun requestCompleted(succeeded: Boolean) {
        onConnectionChanged(observation.requestCompleted(succeeded))
    }

    @Synchronized
    private fun available(network: Network?) {
        if (network != currentNetwork) blocked = false
        currentNetwork = network
        capabilities = network?.let { manager?.getNetworkCapabilities(it) }
        update()
    }

    private fun update() {
        val internet = capabilities?.hasInternet() == true
        val recovered = signal.update(internet && !blocked,
            capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED) == true)
        onConnectionChanged(observation.networkChanged(internet, blocked, recovered))
        if (recovered) {
            com.govorun.lite.stylization.OfficialServiceAvailability.get(context).requestRecovery()
            onNetworkAvailable()
        }
    }
}

internal class NetworkRecoverySignal {
    private val lock = Any()
    private var usable = false
    private var validated = false

    fun update(next: Boolean, validated: Boolean = false): Boolean = synchronized(lock) {
        val recovered = next && (!usable || validated && !this.validated)
        usable = next
        this.validated = next && validated
        recovered
    }
}

private fun NetworkCapabilities.hasInternet(): Boolean =
    hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
