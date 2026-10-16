package com.govorun.lite.account

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import org.json.JSONObject

/** Android jobs for shared profile commands and native sync/device I/O. The
 * common AccountSession owns profile admission, coalescing, freshness and cadence.
 */
internal class AccountProfileRefreshCoordinator(
    private val scope: CoroutineScope,
    private val account: SharedAccountClient,
    private val sessionApi: AccountSessionApi,
    private val publisher: AccountSessionPublisher,
    private val state: MutableStateFlow<AccountState>,
    private val accountSync: AccountSyncCoordinator,
) {
    private var ancillary: Job? = null
    private var ancillaryReference: String? = null

    fun startMaintenance() {
        if (!account.configured) return
        scope.launch {
            while (isActive) {
                try { account.awaitActivity() }
                catch (error: Exception) {
                    if (error is kotlinx.coroutines.CancellationException) throw error
                    return@launch
                }
                runCatching { account.command(JSONObject().put("operation", "refreshForActivity")) }
            }
        }
    }

    fun requestIfStale(accountId: String) {
        if (account.session()?.accountId == accountId) scope.launch {
            runCatching { account.command(JSONObject().put("operation", "refreshForActivity")) }
        }
    }

    fun request() {
        scope.launch {
            runCatching { awaitFreshProfile() }.onSuccess { requestAncillary() }
        }
    }

    suspend fun awaitFreshProfile() {
        val session = account.readableSession() ?: throw AccountSessionChangedException()
        sessionApi.profile(session)
    }

    @Synchronized
    fun requestAncillary() {
        val session = account.session() ?: return
        if (ancillary?.isActive == true && ancillaryReference == session.accessToken) return
        ancillary?.cancel()
        ancillaryReference = session.accessToken
        ancillary = scope.launch {
            val profileGeneration = state.value.profileGeneration
            val syncEnabled = session.accountId?.let(accountSync::isEnabled) == true
            publisher.updateIfCurrent(profileGeneration) { it.copy(syncing = syncEnabled, message = null) }
            runCatching {
                sessionApi.devices(session)
                publisher.publishMetadata(profileGeneration, syncEnabled)
                val synchronized = !syncEnabled || accountSync.synchronizeNow(session)
                publisher.updateSession(session.accessToken, account) {
                    it.copy(syncing = false,
                        message = if (synchronized) null else SYNC_RETRY_MESSAGE)
                }
            }.onFailure { error ->
                if (error is kotlinx.coroutines.CancellationException) throw error
                publisher.updateSession(session.accessToken, account) {
                    it.copy(syncing = false, message = SYNC_RETRY_MESSAGE)
                }
            }
        }
    }
}

internal const val SYNC_RETRY_MESSAGE = "Синхронизация продолжится, когда появится связь"
