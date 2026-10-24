package com.govorun.lite.network

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import com.govorun.lite.storage.GovorunPreferences
import com.govorun.lite.util.StorageKeys
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

enum class NetworkRefusal { OFFLINE, ROAMING }

/**
 * The user's choice to keep Govorun off the network: always, or while roaming.
 * Adapters treat a refused network exactly like a missing one, so dictation
 * keeps its proven offline path. Device-local: never part of settings sync.
 */
internal object NetworkPolicy {
    data class Choice(val offline: Boolean = false, val noNetworkInRoaming: Boolean = true)

    private val mutableChoice = MutableStateFlow<Choice?>(null)

    fun choice(context: Context): StateFlow<Choice?> {
        if (mutableChoice.value == null) mutableChoice.compareAndSet(null, read(context))
        return mutableChoice.asStateFlow()
    }

    fun setOffline(context: Context, offline: Boolean) =
        write(context) { it.copy(offline = offline) }

    fun setNoNetworkInRoaming(context: Context, enabled: Boolean) =
        write(context) { it.copy(noNetworkInRoaming = enabled) }

    /** Why the user refused this network, or null. Missing capabilities mean no network. */
    fun refusal(context: Context, capabilities: NetworkCapabilities?): NetworkRefusal? {
        val choice = choice(context).value ?: read(context)
        return when {
            choice.offline -> NetworkRefusal.OFFLINE
            choice.noNetworkInRoaming && capabilities != null &&
                !capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_ROAMING) -> NetworkRefusal.ROAMING
            else -> null
        }
    }

    fun allows(context: Context, capabilities: NetworkCapabilities?): Boolean =
        refusal(context, capabilities) == null

    fun activeRefusal(context: Context): NetworkRefusal? {
        val manager = context.getSystemService(ConnectivityManager::class.java)
        return refusal(context, manager?.activeNetwork?.let(manager::getNetworkCapabilities))
    }

    fun allowsActiveNetwork(context: Context): Boolean = activeRefusal(context) == null

    private fun read(context: Context): Choice {
        val prefs = prefs(context)
        return Choice(
            offline = prefs.getBoolean(StorageKeys.Key.OFFLINE_MODE, false),
            noNetworkInRoaming = prefs.getBoolean(StorageKeys.Key.NO_NETWORK_IN_ROAMING, true),
        )
    }

    @Synchronized
    private fun write(context: Context, change: (Choice) -> Choice) {
        val next = change(choice(context).value ?: read(context))
        prefs(context).edit()
            .putBoolean(StorageKeys.Key.OFFLINE_MODE, next.offline)
            .putBoolean(StorageKeys.Key.NO_NETWORK_IN_ROAMING, next.noNetworkInRoaming)
            .apply()
        mutableChoice.value = next
    }

    private fun prefs(context: Context) = GovorunPreferences.of(context.applicationContext, StorageKeys.PREFS_FILE)
}
