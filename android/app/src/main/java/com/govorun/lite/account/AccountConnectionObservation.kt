package com.govorun.lite.account

import com.govorun.lite.network.NetworkRefusal

/** Content-free Android network and transport observations, independent of account authority. */
internal class AccountConnectionObservation {
    private var usableNetwork = true
    private var requestFailed = false

    @Synchronized
    fun networkChanged(available: Boolean, blocked: Boolean = false, recovered: Boolean = false): Boolean {
        val next = available && !blocked
        if (next && (!usableNetwork || recovered)) requestFailed = false
        usableNetwork = next
        return usableNetwork && !requestFailed
    }

    @Synchronized
    fun requestCompleted(succeeded: Boolean): Boolean {
        requestFailed = !succeeded
        return usableNetwork && !requestFailed
    }
}

internal const val NO_INTERNET_MESSAGE = "Нет доступа к интернету"
internal const val NO_INTERNET_HINT = "Проверьте подключение и настройки VPN."

internal fun networkRefusalMessage(refusal: NetworkRefusal): String = when (refusal) {
    NetworkRefusal.OFFLINE -> "Офлайн · Выключите в меню"
    NetworkRefusal.ROAMING -> "Офлайн в роуминге · Выключите в настройках"
}

/** Localized display of OS connectivity and the common owner's profile freshness. */
internal val AccountState.connectionMessage: String?
    get() = when {
        !networkAvailable && networkRefusal != null -> networkRefusalMessage(networkRefusal)
        !networkAvailable -> NO_INTERNET_MESSAGE
        signedIn && !profileFresh -> "Не удалось проверить доступ к улучшению текста"
        else -> null
    }

internal val AccountState.connectionHint: String
    get() = if (!networkAvailable && networkRefusal != null) ""
        else if (!networkAvailable) NO_INTERNET_HINT
        else "Не удалось связаться с сервером. Повторите попытку."
