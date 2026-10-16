package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.network.OfficialSessionNetworkGate
import com.govorun.lite.stylization.StylizationDiagnosticsStore
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.flow.MutableStateFlow
import org.json.JSONObject

/** Native projection and OS effects. It never saves credentials, rotates tokens,
 * accepts a profile or decides whether the account is authorized.
 */
internal class AccountSessionPublisher(
    context: Context,
    private val configured: Boolean,
    private val transitions: AccountSessionTransitionGate,
    private val state: MutableStateFlow<AccountState>,
    private val resolveSync: (GovorunSession) -> AccountDataSyncMode,
    private val onStylizationDisabled: () -> Unit = {},
) {
    private val appContext = context.applicationContext
    private var coreProfileRevision = -1L
    private var coreGeneration = -1L
    private var signedOutApplied = false
    private var previousProjection: JSONObject? = null

    fun project(value: JSONObject, session: GovorunSession?,
        decorate: (AccountState) -> AccountState = { it }) = transitions.locked {
        val snapshot = value.getJSONObject("snapshot")
        val epoch = snapshot.getLong("sessionGeneration")
        val profileRevision = snapshot.getLong("profileRevision")
        val previous = state.value
        val reserveAction = com.govorun.lite.shared.SharedBusiness.evaluate(JSONObject()
            .put("operation", "styleReserveTransition")
            .put("previous", previousProjection ?: JSONObject.NULL).put("current", value)).getString("action")
        val changedIdentity = epoch != coreGeneration
        val changedProfile = profileRevision != coreProfileRevision
        if (changedIdentity || changedProfile) transitions.advance { }
        coreGeneration = epoch
        coreProfileRevision = profileRevision
        val generation = transitions.currentGeneration()
        val storageAvailable = snapshot.getBoolean("storageAvailable")
        val problem = if (storageAvailable) null else SecureStorageProblem.TEMPORARILY_UNAVAILABLE
        val authority = snapshot.getJSONObject("authority")
        val commonAuthority = AccountAuthority(authority.getBoolean("entitlementActive"),
            authority.getBoolean("authorized"), authority.getBoolean("processingReady"),
            AccountAccessPresentation.read(snapshot.getJSONObject("access")))
        if (!snapshot.getBoolean("signedIn")) {
            OfficialSessionNetworkGate.revokeAll()
            val effects = runCatching {
                if (reserveAction == "clear" && !signedOutApplied) {
                    com.govorun.lite.notifications.NotificationRepository.onSignedOut()
                    Prefs.setStylizationEnabled(appContext, false)
                    onStylizationDisabled()
                    StylizationDiagnosticsStore.clear(appContext)
                    signedOutApplied = true
                }
            }
            if (effects.isSuccess) previousProjection = JSONObject(value.toString())
            val pending = value.optJSONObject("pendingDeviceLimit")
            state.value = AccountState(available = configured, profileGeneration = generation,
                networkAvailable = previous.networkAvailable, profileFresh = snapshot.getBoolean("profileFresh"),
                sessionGeneration = epoch, syncing = snapshot.getBoolean("signingIn"),
                secureStorageProblem = problem ?: SecureStorageProblem.TEMPORARILY_UNAVAILABLE.takeIf { effects.isFailure },
                deviceLimitReached = pending != null,
                deviceLimit = pending?.optInt("deviceLimit", 5) ?: 5,
                devices = pending?.optJSONArray("devices")?.let(::parseDevices).orEmpty(),
                message = problem?.userMessage)
            return@locked
        }
        signedOutApplied = false
        val current = checkNotNull(session)
        previousProjection = JSONObject(value.toString())
        val deviceList = value.optJSONObject("devices")
        val next = publishedAccountState(configured, current,
            syncing = previous.syncing,
            devices = deviceList?.getJSONArray("devices")?.let(::parseDevices).orEmpty()).copy(
            deviceLimit = deviceList?.getInt("deviceLimit") ?: current.deviceLimit,
            authority = commonAuthority,
            networkAvailable = previous.networkAvailable, profileFresh = snapshot.getBoolean("profileFresh"),
            profileGeneration = generation, sessionGeneration = epoch,
            secureStorageProblem = problem,
            dataSyncMode = if (changedIdentity || changedProfile) resolveSync(current) else previous.dataSyncMode,
            styleWindowJson = if (changedIdentity) null else previous.styleWindowJson,
            localUnspentStyleCredits = if (changedIdentity) 0 else previous.localUnspentStyleCredits,
            localPendingStyleCredits = if (changedIdentity) 0 else previous.localPendingStyleCredits,
            localStylePassIssuancePending = !changedIdentity && previous.localStylePassIssuancePending,
            localStylePassStateKnown = !changedIdentity && previous.localStylePassStateKnown,
            pendingStyleUsageToday = if (changedIdentity) 0 else previous.pendingStyleUsageToday,
            pendingStyleUsageTotal = if (changedIdentity) 0 else previous.pendingStyleUsageTotal,
        )
        // Publish the matching account and style window together. Observers must
        // not see the intermediate account-only state during a profile refresh.
        state.value = decorate(next)
    }

    fun publishMetadata(expectedGeneration: Long, syncing: Boolean) {
        updateIfCurrent(expectedGeneration) { it.copy(syncing = syncing) }
    }

    fun updateSession(reference: String, account: AccountSessionReader,
        update: (AccountState) -> AccountState) = transitions.locked {
        if (account.session()?.accessToken == reference) {
            state.value = update(state.value)
        }
    }

    fun updateIfCurrent(expectedGeneration: Long, update: (AccountState) -> AccountState) {
        transitions.ifCurrent(expectedGeneration) { state.value = update(state.value) }
    }
}
