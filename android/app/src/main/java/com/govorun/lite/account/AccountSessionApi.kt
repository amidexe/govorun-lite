package com.govorun.lite.account

/** Native DTO adapter. Authorization/rotation/retry are owned by AccountSession. */
internal class AccountSessionApi(
    private val account: AccountSessionReader,
    val api: AccountApiClient,
) {
    suspend fun <T> authorizedCurrent(call: suspend (String) -> T): T =
        authorized(account.readableSession() ?: throw AccountSessionChangedException(), call)

    suspend fun profile(session: GovorunSession): GovorunSession {
        authorized(session) { token -> api.documentRequest(token, org.json.JSONObject().put("operation", "accountProfile")) }
        return currentSession(session)
    }

    suspend fun devices(session: GovorunSession): List<AccountDevice> =
        authorized(session) { token -> api.devices(token, NeutralDeviceName.current()).second }

    suspend fun <T> authorized(session: GovorunSession, call: suspend (String) -> T): T {
        val current = currentSession(session)
        val result = call(current.accessToken)
        currentSession(session)
        return result
    }

    private fun currentSession(expected: GovorunSession): GovorunSession {
        val current = account.readableSession() ?: throw AccountSessionChangedException()
        if (!sameLogicalAccount(expected, current)) throw AccountSessionChangedException()
        return current
    }
}

internal class AccountSessionChangedException : Exception("Account session changed")

internal const val ACCOUNT_SESSION_ENDED_MESSAGE =
    "Сессия на этом устройстве завершена. Войдите снова."

/** Opaque references include the common owner's lifetime and identity epoch. */
internal fun sameLogicalAccount(first: GovorunSession, second: GovorunSession): Boolean =
    first.accessToken == second.accessToken && first.refreshToken == second.refreshToken

internal fun AccountSessionReader.hasSession(expected: GovorunSession): Boolean =
    session()?.let { sameLogicalAccount(expected, it) } == true
