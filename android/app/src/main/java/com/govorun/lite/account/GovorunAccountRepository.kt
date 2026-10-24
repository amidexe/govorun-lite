package com.govorun.lite.account
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import com.govorun.lite.BuildConfig
import com.govorun.lite.network.NetworkPolicy
import com.govorun.lite.stylization.StylizationTerms
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
object GovorunAccountRepository {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val sessionTransitions = AccountSessionTransitionGate()
    private val mutableState = MutableStateFlow(AccountState())
    val state: StateFlow<AccountState> = mutableState.asStateFlow()
    private val mutableBillingActions = MutableStateFlow(BillingActions.None)
    internal val billingActions = mutableBillingActions.asStateFlow()
    internal val billingBoundary = billingSessionBoundary(sessionTransitions) { state.value }
    @Volatile private var initialized = false
    private lateinit var appContext: Context
    private lateinit var secure: SecureAccountStorage
    private lateinit var account: SharedAccountClient
    internal suspend fun sharedAccountCommand(command: org.json.JSONObject) = account.command(command)
    internal fun sharedAccountProjection() = account.projection
    internal fun reserveAccountAuthIntent() = account.reserveAuthIntent()
    private lateinit var stats: StatsSyncStore
    private lateinit var api: AccountApiClient
    private lateinit var sessionApi: AccountSessionApi
    private lateinit var sessionPublisher: AccountSessionPublisher
    private lateinit var stylesProjection: SharedStylesProjection
    private lateinit var preferences: PortablePreferenceSyncStore
    private lateinit var contentStore: AccountContentSyncStore
    private lateinit var content: AccountContentSyncCoordinator
    private lateinit var syncEnrollment: AccountSyncEnrollmentStore
    private lateinit var dataSyncController: AccountDataSyncController
    private lateinit var accountSync: AccountSyncCoordinator
    private lateinit var profileRefresh: AccountProfileRefreshCoordinator
    private lateinit var storageRecovery: AccountSecureStorageRecovery
    private var networkRecovery: AccountNetworkRecovery? = null
    @Synchronized fun initialize(context: Context) {
        if (initialized) return
        appContext = context.applicationContext
        secure = SecureAccountStorage(appContext)
        stats = StatsSyncStore(appContext)
        stylesProjection = SharedStylesProjection(sessionTransitions, mutableState)
        val styles = SharedStylesEffects(appContext, stylesProjection::project,
            com.govorun.lite.stylization.OfficialServiceAvailability::project, { _, _ -> })
        account = SharedAccountClient(scope, BuildConfig.ACCOUNT_API_URL, StylizationTerms.CURRENT_VERSION,
            SharedAccountStorage(appContext),
            onProjection = { projection, session ->
                sessionPublisher.project(projection, session, stylesProjection::decorate)
                BillingRepository.acceptAccountProjection(projection)
                mutableBillingActions.value = BillingActions.read(projection, state.value.sessionGeneration)
            },
            onUnavailable = {
                mutableBillingActions.value = BillingActions.None
                mutableState.value = AccountState(available = false,
                    secureStorageProblem = SecureStorageProblem.TEMPORARILY_UNAVAILABLE)
            },
            transport = AccountHttpTransport(BuildConfig.ACCOUNT_API_URL,
                onNetworkResult = { networkRecovery?.requestCompleted(it) },
                networkAllowed = { NetworkPolicy.allowsActiveNetwork(appContext) }), styles = styles::execute)
        api = AccountApiClient(account, account.configured)
        sessionApi = AccountSessionApi(account, api)
        preferences = PortablePreferenceSyncStore(appContext)
        contentStore = AccountContentSyncStore(appContext)
        syncEnrollment = AccountSyncEnrollmentStore(appContext)
        content = AccountContentSyncCoordinator(
            account, api, contentStore,
        )
        accountSync = AccountSyncCoordinator(
            appContext, scope, account, stats, preferences, content, syncEnrollment,
            api, { accountId -> profileRefresh.requestIfStale(accountId) },
        )
        dataSyncController = AccountDataSyncController(
            appContext, scope, account, stats, preferences, contentStore,
            syncEnrollment, accountSync, mutableState,
        )
        sessionPublisher = AccountSessionPublisher(appContext, api.configured,
            sessionTransitions, mutableState,
            resolveSync = { session -> session.accountId?.let { dataSyncController.resolve(it, session.hasSyncedData) }
                ?: AccountDataSyncMode.EVALUATING },
            onStylizationDisabled = { onStylizationPreferenceChanged() })
        profileRefresh = AccountProfileRefreshCoordinator(scope, account, sessionApi,
            sessionPublisher, mutableState, accountSync)
        storageRecovery = AccountSecureStorageRecovery(scope, account) {
            profileRefresh.request(); sendStyles("stylizationStart")
        }
        initialized = true
        networkRecovery = AccountNetworkRecovery(appContext, onConnectionChanged = { connected ->
            sendStyles("stylizationConnectivity") { put("online", connected) }
            val refusal = NetworkPolicy.activeRefusal(appContext)
            sessionTransitions.locked {
                mutableState.value = mutableState.value.copy(networkAvailable = connected, networkRefusal = refusal)
            }
        }) {
            if (account.storageProblem != null) storageRecovery.request()
            if (account.session() != null) {
                profileRefresh.request()
                accountSync.request(0)
                sendStyles("stylizationStart")
            }
        }.also(AccountNetworkRecovery::start)
        scope.launch { NetworkPolicy.choice(appContext).drop(1).collect { networkRecovery?.policyChanged() } }
        if (account.configured) account.start()
        profileRefresh.startMaintenance()
    }

    fun startLogin(activity: Activity, provider: AuthProvider) {
        ensureInitialized(activity)
        val intent = account.reserveAuthIntent()
        scope.launch {
            try {
                val url = account.begin(provider, intent)
                kotlinx.coroutines.withContext(Dispatchers.Main) {
                    if (!account.isCurrentIntent(intent)) return@withContext
                    activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
                }
            } catch (error: Exception) {
                if (error is kotlinx.coroutines.CancellationException) throw error
                if (account.cancelIfCurrent(intent)) {
                    mutableState.value = mutableState.value.copy(syncing = false,
                        message = "Не удалось открыть вход. Повторите попытку.")
                }
            }
        }
    }

    fun clearAbandonedLogin() {
        if (initialized) scope.launch {
            runCatching { account.command(org.json.JSONObject().put("operation", "authAbandon")) }
        }
    }

    suspend fun completeAuthCallback(context: Context, callback: Uri): Boolean {
        ensureInitialized(context)
        val response = try { account.callback(callback.toString(), secure.installationId(), NeutralDeviceName.current()) }
        catch (error: Exception) {
            if (error is kotlinx.coroutines.CancellationException) throw error
            return false
        }
        if (response.getInt("status") != 200) return false
        profileRefresh.requestAncillary()
        sendStyles("stylizationStart")
        return true
    }

    fun replaceDeviceDuringLogin(deviceId: String) {
        if (initialized) scope.launch {
            runCatching { account.replaceDevice(deviceId, secure.installationId(), NeutralDeviceName.current()) }
                .onSuccess { if (it.getInt("status") == 200) profileRefresh.requestAncillary() }
                .onFailure { mutableState.value = mutableState.value.copy(syncing = false,
                    message = "Не удалось заменить устройство. Повторите попытку.") }
        }
    }

    fun revokeDevice(deviceId: String) {
        if (!initialized) return
        scope.launch {
            val operation = sessionTransitions.locked { generation ->
                account.session()?.let { it to generation }
            } ?: return@launch
            val (session, expectedGeneration) = operation
            sessionPublisher.updateIfCurrent(expectedGeneration) {
                it.copy(syncing = true, message = null)
            }
            runCatching {
                // The shared command may intentionally end this session.
                api.revokeDevice(session.accessToken, deviceId)
                sessionPublisher.updateIfCurrent(expectedGeneration) { it.copy(syncing = false) }
            }.onFailure {
                sessionPublisher.updateIfCurrent(expectedGeneration) { state ->
                    state.copy(syncing = false, message = "Не удалось отключить устройство")
                }
            }
        }
    }
    @Synchronized
    fun refreshAccount() {
        if (initialized) {
            if (account.storageProblem != null) storageRecovery.request() else profileRefresh.request()
        }
    }
    fun signOut() {
        if (!initialized) return
        val intent = account.reserveAuthIntent()
        scope.launch {
            com.govorun.lite.notifications.NotificationRepository.beforeSignOut()
            runCatching { account.logout(intent) }.onFailure {
                com.govorun.lite.notifications.NotificationRepository.onSignedOut()
                mutableState.value = mutableState.value.copy(message = "Не удалось сохранить выход. Повторите попытку.")
            }
        }
    }
    fun mergeLocalDataIntoAccount() {
        if (initialized) dataSyncController.mergeLocalDataIntoAccount()
    }
    fun keepLocalDataOnly() {
        if (initialized) dataSyncController.keepLocalDataOnly()
    }
    fun requestDataSyncDecision() {
        if (initialized) dataSyncController.requestDecision()
    }
    fun deleteAccount() {
        if (!initialized) return
        scope.launch {
            val generation = sessionTransitions.currentGeneration()
            sessionPublisher.updateIfCurrent(generation) { it.copy(deletingAccount = true, message = null) }
            val session = account.readableSession() ?: return@launch
            runCatching { api.deleteAccount(session.accessToken) }
                .onFailure { sessionPublisher.updateIfCurrent(generation) { state ->
                    state.copy(deletingAccount = false, message = "Не удалось подтвердить удаление. Повторите попытку.")
                } }
        }
    }
    internal suspend fun <T> withAuthorizedAccount(
        block: suspend (AccountApiClient, String) -> T,
    ): T {
        check(initialized && api.configured)
        return sessionApi.authorizedCurrent { token -> block(api, token) }
    }
    suspend fun activateStarterGrant(
        context: Context,
        acceptCurrentTerms: Boolean,
        preferredStyle: String,
        promoProgram: PromoProgram = state.value.promoProgram,
    ): Boolean {
        runAccountAccess(context, org.json.JSONObject().put("operation", "activateFreeAccess")
            .put("activation", org.json.JSONObject().put("confirmed", true)
                .put("termsVersion", StylizationTerms.CURRENT_VERSION.takeIf { acceptCurrentTerms })
                .put("preferredStyle", preferredStyle)
                .put("expectedProgramVersion", promoProgram.programVersion.takeIf { promoProgram.visible })))
        return true
    }
    suspend fun acceptOnlineTerms(context: Context): Boolean {
        runAccountAccess(context, org.json.JSONObject().put("operation", "acceptOnlineTerms")
            .put("version", StylizationTerms.CURRENT_VERSION).put("confirmed", true))
        return true
    }
    private suspend fun runAccountAccess(context: Context, command: org.json.JSONObject) {
        ensureInitialized(context)
        account.command(command.put("sessionGeneration", state.value.sessionGeneration))
    }

    fun recordStatsDayChanged(context: Context, date: String) {
        ensureInitialized(context)
        dataSyncController.recordStatsDayChanged(date)
    }
    fun recordPortablePreferenceChange(
        context: Context,
        defaultStyle: String? = null,
        reducedPunctuation: Boolean? = null,
        punctuationMode: String? = null,
        emojiEnabled: Boolean? = null,
        emojiMode: String? = null,
        profanityFilterEnabled: Boolean? = null,
        shortPhraseThreshold: Int? = null,
    ) {
        ensureInitialized(context)
        dataSyncController.recordPortablePreferenceChange(defaultStyle, reducedPunctuation,
            punctuationMode, emojiEnabled, emojiMode, profanityFilterEnabled, shortPhraseThreshold)
    }
    fun recordSyncedContentChange(context: Context) {
        ensureInitialized(context)
        dataSyncController.recordSyncedContentChange()
    }
    internal fun <T> withSyncedContentMutation(context: Context, action: () -> T): T {
        ensureInitialized(context)
        return dataSyncController.withContentMutation(action)
    }
    internal fun cloudDays(): List<StatsDay> {
        if (!initialized) return emptyList()
        return dataSyncController.cloudDays()
    }
	internal fun cloudTotal(): StatsLifetimeTotal? {
		if (!initialized) return null
		return dataSyncController.cloudTotal()
	}
    fun refreshStyleCreditBalance() = sendStyles("stylizationStart")
    internal fun onStylizationPreferenceChanged() = sendStyles("stylizationSettingsChanged") { put("invalidateScope", false) }
    internal suspend fun joinDictationPreparation(requiredCredits: Int): Long =
        sharedAccountCommand(org.json.JSONObject().put("operation", "stylizationJoinPreparation")
            .put("sessionGeneration", state.value.sessionGeneration).put("requiredCredits", requiredCredits))
            .getLong("elapsedMilliseconds")
    internal fun prepareOfficialStylePassesForUpcomingDictation() = sendStyles("stylizationStart")
    internal fun onStylizationConsentScopeChanged() = sendStyles("stylizationSettingsChanged") { put("invalidateScope", true) }
    internal fun recordStyleFailure(reason: String) = sendStyles("stylizationFailure") { put("reason", reason) }
    internal fun recoverStyles() = sendStyles("stylizationRecover")
    private fun sendStyles(operation: String, fields: org.json.JSONObject.() -> Unit = {}) {
        if (initialized && account.configured) scope.launch {
            try { sharedAccountCommand(org.json.JSONObject().put("operation", operation)
                .apply { if (operation != "stylizationStart") put("sessionGeneration", account.generation); fields() }) }
            catch (cancelled: kotlinx.coroutines.CancellationException) { throw cancelled }
            catch (_: Exception) { /* Core publishes status; user text is never replayed here. */ }
        }
    }
    fun requestSynchronizedStatsReset(context: Context) {
        ensureInitialized(context)
        dataSyncController.requestSynchronizedStatsReset()
    }
    private fun ensureInitialized(context: Context) {
        if (!initialized) initialize(context)
    }
}
