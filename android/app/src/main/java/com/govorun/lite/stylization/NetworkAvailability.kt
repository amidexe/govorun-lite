package com.govorun.lite.stylization

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities

internal object NetworkAvailability {
    fun hasUsableNetwork(context: Context): Boolean {
        val manager = context.getSystemService(ConnectivityManager::class.java) ?: return false
        val network = manager.activeNetwork ?: return false
        val capabilities = manager.getNetworkCapabilities(network) ?: return false
        // VALIDATED is only a momentary Android probe result. Slow Wi-Fi and
        // handovers can clear it while an HTTPS exchange is still viable, so
        // the request itself must decide success/failure once a network with
        // Internet capability exists.
        return shouldAttemptStylizationRequest(
            hasActiveNetwork = true,
            hasInternetCapability = capabilities.hasCapability(
                NetworkCapabilities.NET_CAPABILITY_INTERNET,
            ),
            isValidated = capabilities.hasCapability(
                NetworkCapabilities.NET_CAPABILITY_VALIDATED,
            ),
        )
    }

}

internal fun shouldAttemptStylizationRequest(
    hasActiveNetwork: Boolean,
    hasInternetCapability: Boolean,
    @Suppress("UNUSED_PARAMETER") isValidated: Boolean,
): Boolean = hasActiveNetwork && hasInternetCapability
