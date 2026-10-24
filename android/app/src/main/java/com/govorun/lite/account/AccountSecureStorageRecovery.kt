package com.govorun.lite.account

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch

/** Retries the same encrypted record; never creates an identity or resets the pass vault. */
internal class AccountSecureStorageRecovery(
    private val scope: CoroutineScope,
    private val account: SharedAccountClient,
    private val onRecovered: () -> Unit,
) {
    private var job: Job? = null

    @Synchronized
    fun request() {
        if (!account.configured || job?.isActive == true) return
        job = scope.launch {
            var attempt = 0
            while (true) {
                val schedule = try { account.command(org.json.JSONObject().put("operation", "storageRecoveryDelay")
                    .put("attempt", attempt++)) }
                catch (error: Exception) {
                    if (error is kotlinx.coroutines.CancellationException) throw error
                    return@launch
                }
                if (schedule.isNull("milliseconds")) return@launch
                delay(schedule.getLong("milliseconds"))
                // Before the first projection every start looks unavailable; a record
                // read by then must not be written again.
                if (account.storageProblem != null) runCatching { account.restore() }
                if (account.storageProblem == null) {
                    if (account.session() != null) onRecovered()
                    return@launch
                }
            }
        }
    }
}

internal fun storedAccountUnavailable(state: AccountState, problem: SecureStorageProblem) =
    state.copy(secureStorageProblem = problem, syncing = false, message = problem.userMessage)

internal fun restoredStoredAccountState(
    previous: AccountState,
    read: SecureRecordRead<GovorunSession>,
    configured: Boolean,
    generation: Long,
    resolveSync: (String) -> AccountDataSyncMode,
): AccountState = when (read) {
    is SecureRecordRead.Unavailable -> storedAccountUnavailable(previous, read.problem)
    SecureRecordRead.Missing -> AccountState(available = configured, profileGeneration = generation)
    is SecureRecordRead.Available -> {
        val session = read.value
        publishedAccountState(configured, session, syncing = false, devices = previous.devices).copy(
            dataSyncMode = session.accountId?.let(resolveSync) ?: AccountDataSyncMode.EVALUATING,
            profileGeneration = generation,
            sessionGeneration = generation,
            localUnspentStyleCredits = previous.localUnspentStyleCredits,
            localPendingStyleCredits = previous.localPendingStyleCredits,
            localStylePassIssuancePending = previous.localStylePassIssuancePending,
            localStylePassStateKnown = previous.localStylePassStateKnown,
            pendingStyleUsageToday = previous.pendingStyleUsageToday,
            pendingStyleUsageTotal = previous.pendingStyleUsageTotal,
        )
    }
}
