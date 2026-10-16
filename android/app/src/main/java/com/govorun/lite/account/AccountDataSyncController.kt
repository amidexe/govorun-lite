package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.stats.StatisticsJournal
import org.json.JSONObject
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch

internal class AccountDataSyncController(
    context: Context,
    private val scope: CoroutineScope,
    private val secure: AccountSessionReader,
    private val stats: StatsSyncStore,
    private val preferences: PortablePreferenceSyncStore,
    private val contentStore: AccountContentSyncStore,
    private val enrollment: AccountSyncEnrollmentStore,
    private val sync: AccountSyncCoordinator,
    private val state: MutableStateFlow<AccountState>,
) {
    private val appContext = context.applicationContext

    fun resolve(accountId: String): AccountDataSyncMode = resolve(accountId, null)

    fun resolve(accountId: String, hasRemoteData: Boolean?): AccountDataSyncMode = enrollment.resolve(
        accountId = accountId,
        localContent = contentStore.localDocument(),
        statistics = StatisticsJournal.projection(appContext).let { local ->
            JSONObject().put("total", local.getJSONObject("total")).put("days", local.getJSONArray("days"))
        },
        hasExistingSyncHistory = contentStore.hasAccountHistory(accountId) ||
            stats.hasAccountHistory(accountId) || preferences.hasAccountHistory(accountId),
        hasRemoteData = hasRemoteData,
    )

    fun mergeLocalDataIntoAccount() {
        val session = secure.session() ?: return
        val accountId = session.accountId ?: return
        preferences.enqueueLocalValues(accountId)
        enrollment.enable(accountId)
        state.value = state.value.copy(
            dataSyncMode = AccountDataSyncMode.ENABLED,
            syncing = true,
            message = null,
        )
        scope.launch {
            val succeeded = sync.synchronizeNow(session)
            if (!secure.hasSession(session)) return@launch
            state.value = state.value.copy(
                syncing = false,
                message = if (succeeded) null else SYNC_RETRY_MESSAGE,
            )
        }
    }

    fun keepLocalDataOnly() {
        val accountId = secure.session()?.accountId ?: return
        enrollment.keepLocal(accountId)
        state.value = state.value.copy(
            dataSyncMode = AccountDataSyncMode.LOCAL_ONLY,
            syncing = false,
            message = null,
        )
    }

    fun requestDecision() {
        val accountId = secure.session()?.accountId ?: return
        enrollment.requestDecision(accountId)
        state.value = state.value.copy(
            dataSyncMode = AccountDataSyncMode.PENDING,
            syncing = false,
            message = null,
        )
    }

    fun recordStatsDayChanged(date: String) {
        enabledAccountId() ?: return
        sync.request()
    }

    fun recordPortablePreferenceChange(
        defaultStyle: String? = null,
        reducedPunctuation: Boolean? = null,
        punctuationMode: String? = null,
        emojiEnabled: Boolean? = null,
        emojiMode: String? = null,
        profanityFilterEnabled: Boolean? = null,
        shortPhraseThreshold: Int? = null,
    ) {
        val accountId = enabledAccountId() ?: return
        preferences.enqueue(
            accountId,
            PortablePreferenceChanges(
                defaultStyle,
                reducedPunctuation,
                punctuationMode = punctuationMode,
                emojiEnabled = emojiEnabled,
                emojiMode = emojiMode,
                profanityFilterEnabled = profanityFilterEnabled,
                shortPhraseThreshold = shortPhraseThreshold,
            ),
        )
        sync.request()
    }

    fun recordSyncedContentChange() {
        val accountId = enabledAccountId()
        contentStore.markLocalChange(accountId)
        if (accountId != null) sync.request()
    }

    fun <T> withContentMutation(action: () -> T): T = contentStore.withLocalMutation(action)

    fun cloudDays(): List<StatsDay> = enabledAccountId()?.let(stats::cloudDays).orEmpty()

    fun cloudTotal(): StatsLifetimeTotal? = enabledAccountId()?.let(stats::cloudTotal)

    fun requestSynchronizedStatsReset() {
        val accountId = enabledAccountId()
        stats.reset(accountId)
        if (accountId != null) sync.request()
    }

    private fun enabledAccountId(): String? = secure.session()?.accountId?.takeIf(sync::isEnabled)
}
