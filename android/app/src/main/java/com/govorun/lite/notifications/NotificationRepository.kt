package com.govorun.lite.notifications

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import androidx.core.app.NotificationManagerCompat
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.GovorunAccountRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.distinctUntilChanged
import org.json.JSONObject
import java.security.SecureRandom

data class NotificationUiState(
    val showInvitation: Boolean = false,
    val configured: Boolean = false,
    val supported: Boolean = false,
    val choices: NotificationChoices = NotificationChoices(),
    val prompted: Boolean = false,
    val permission: Boolean = false,
    val pending: Boolean = false,
    val storageAvailable: Boolean = true,
    val notices: List<UpdateNotice> = emptyList(),
)

object NotificationRepository {
    private lateinit var context: Context
    private lateinit var storage: NotificationStorage
    private lateinit var transport: PushTransport
    private val api = NotificationRegistrationApi()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val network = Mutex()
    private var value = NotificationState()
    private var storageAvailable = true
    @Volatile private var signingOut = false
    private var signedOutGeneration: Long? = null
    private var work: Job? = null
    private val mutableUi = MutableStateFlow(NotificationUiState())
    val ui = mutableUi.asStateFlow()

    @Synchronized
    fun initialize(app: Context) {
        if (::context.isInitialized) return
        context = app.applicationContext
        storage = NotificationStorage(context)
        transport = FirebasePushTransport(context)
        try { value = storage.read() } catch (_: Exception) { storageAvailable = false }
        val firstSeen = value.withFirstSeenAt(now())
        if (firstSeen != value) save(firstSeen)
        refresh()
        scope.launch {
            GovorunAccountRepository.state.map { notificationAccountScope(it, BuildConfig.ACCOUNT_API_URL) to it.sessionGeneration }
                .distinctUntilChanged().collect { refresh() }
        }
    }

    @Synchronized
    fun later() { save(value.changed("later")!!) }

    @Synchronized
    internal fun consentRequest(): NotificationConsentRequest? {
        enforceAccount()
        return if (storageAvailable && value.shouldInvite(api.configured && transport.available(), accountScope(), now())) {
            NotificationConsentRequest(value.accountScope, value.revision)
        } else null
    }

    /** The OS dialog may finish after logout. Its original consent scope must still match. */
    @Synchronized
    internal fun allow(request: NotificationConsentRequest) {
        enforceAccount()
        var next = value.changed("accept", accountScope(), permission(), extra = JSONObject().put("request", request.wire())
            .put("storageAvailable", storageAvailable).put("configured", api.configured).put("supported", transport.available())) ?: return
        if (next.id.isEmpty()) next = next.copy(id = randomHex(16), credential = randomHex(32))
        if (save(next)) {
            refresh()
        }
    }

    @Synchronized
    internal fun decline(request: NotificationConsentRequest) {
        if (value.canAccept(request, accountScope())) later()
    }

    /** One bounded revocation while the explicit logout still owns its session. */
    suspend fun beforeSignOut() {
        val pending = synchronized(this) {
            if (!::context.isInitialized) return
            val signedIn = accountScope().isNotEmpty()
            signingOut = true
            signedOutGeneration = null
            NotificationSyncJob.cancel(context)
            work?.cancel()
            api.cancel()
            val next = value.withAccount("", permission = false)
            if (next != value) save(next)
            value.takeIf { signedIn && storageAvailable && api.configured && it.id.isNotEmpty() }
        } ?: return
        withTimeoutOrNull(2000) {
            try {
                network.withLock {
                    api.sync(pending)
                    synchronized(this@NotificationRepository) {
                        val next = value.changed("acknowledged", now = now(), extra = JSONObject().put("expectedRevision", pending.revision))!!
                        if (next != value) save(next)
                    }
                }
            } catch (_: Exception) { currentCoroutineContext().ensureActive() }
        }
    }

    /** Terminal transitions never start network cleanup after the session ends. */
    @Synchronized
    fun onSignedOut() {
        if (!::context.isInitialized) return
        val next = value.withAccount("", permission = false)
        if (next != value) save(next)
        signingOut = true
        signedOutGeneration = GovorunAccountRepository.state.value.sessionGeneration
        NotificationSyncJob.cancel(context)
        work?.cancel()
        api.cancel()
        cancelDisallowed()
    }

    @Synchronized
    fun refresh() {
        if (!::context.isInitialized) return
        if (signedOutGeneration?.let { it != GovorunAccountRepository.state.value.sessionGeneration } == true) {
            signingOut = false
            signedOutGeneration = null
        }
        enforceAccount()
        publish()
        scheduleSync()
    }

    private fun scheduleSync() {
        if (signingOut || accountScope().isEmpty()) { NotificationSyncJob.cancel(context); return }
        val plan = value.decision("syncPlan", extra = JSONObject().put("storageAvailable", storageAvailable).put("configured", api.configured))
        if (!plan.getBoolean("schedule")) return
        NotificationSyncJob.schedule(context, plan.getBoolean("urgent"))
        if (work?.isActive != true) work = scope.launch { reconcile() }
    }

    internal suspend fun reconcile(): Boolean = network.withLock {
        if (signingOut || accountScope().isEmpty()) return@withLock true
        val plan = synchronized(this) { value.decision("syncPlan", extra = JSONObject().put("storageAvailable", storageAvailable).put("configured", api.configured)) }
        if (!plan.getBoolean("reconcile")) return@withLock true
        // Offline or roaming choice: keep the change pending for the daily job.
        if (!com.govorun.lite.network.NetworkPolicy.allowsActiveNetwork(context)) return@withLock true
        try {
            var cleanupFailed = false
            var snapshot = snapshot()
            if (snapshot.decision("syncPlan").getBoolean("token")) {
                val token = withTimeout(12000) { transport.token() }
                synchronized(this) {
                    // A revocation can happen while Firebase is replying. Never
                    // register that late token under the old consent revision.
                    enforceAccount()
                    val next = value.changed("token", accountScope(), permission(), now(), JSONObject()
                        .put("expectedRevision", snapshot.revision).put("token", token).put("version", BuildConfig.VERSION_CODE.toLong()))!!
                    if (next != value) check(save(next))
                    snapshot = value
                }
            }
            if (signingOut || accountScope().isEmpty()) return@withLock true
            if (snapshot.decision("syncPlan").getBoolean("remove")) {
                try {
                    withTimeout(12000) { transport.remove() }
                    synchronized(this) {
                        val next = value.changed("removed", extra = JSONObject().put("expectedRevision", snapshot.revision))!!
                        if (next != value) check(save(next))
                    }
                } catch (_: Exception) { currentCoroutineContext().ensureActive(); cleanupFailed = true }
            }
            snapshot = snapshot()
            if (signingOut || accountScope().isEmpty()) return@withLock true
            if (snapshot.decision("syncPlan").getBoolean("sync")) {
                if (snapshot.decision("syncPlan").getBoolean("missingToken")) return@withLock false
                api.sync(snapshot)
                synchronized(this) {
                    val next = value.changed("acknowledged", now = now(), extra = JSONObject().put("expectedRevision", snapshot.revision))!!
                    if (next != value) check(save(next))
                }
            }
            snapshot = snapshot()
            if (snapshot.decision("syncPlan").getBoolean("cancel")) {
                NotificationSyncJob.cancel(context)
            }
            synchronized(this) { publish() }
            if (cleanupFailed) NotificationSyncJob.retry(context)
            !cleanupFailed
        } catch (_: Exception) {
            currentCoroutineContext().ensureActive()
            if (signingOut || accountScope().isEmpty()) return@withLock true
            synchronized(this) { publish() }
            NotificationSyncJob.retry(context)
            false
        }
    }

    @Synchronized
    internal fun receive(data: Map<String, String>): UpdateNotice? {
        enforceAccount()
        if (!storageAvailable || !value.mayDeliver(accountScope(), permission())) return null
        val notice = UpdateNotice.parse(data, BuildConfig.APPLICATION_ID, BuildConfig.VERSION_CODE.toLong(), now()) ?: return null
        val accepted = value.receive(notice, permission(), now()) ?: return null
        // Persist the receipt before Android sees it; redelivery after a restart
        // cannot show the same message twice.
        return if (save(accepted)) notice else null
    }

    @Synchronized
    internal fun mayShow(notice: UpdateNotice) = storageAvailable &&
        value.mayDeliver(accountScope(), permission()) && value.choices.accepts(notice.category)

    fun createChannels() {
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("govorun_messages", "Уведомления Говоруна", NotificationManager.IMPORTANCE_DEFAULT)
            .apply { setSound(null, null); enableVibration(false) })
        manager.deleteNotificationChannel("govorun_updates")
        manager.deleteNotificationChannel("govorun_offers")
    }

    private fun permission(): Boolean {
        val manager = context.getSystemService(NotificationManager::class.java)
        return NotificationManagerCompat.from(context).areNotificationsEnabled() &&
            manager.getNotificationChannel("govorun_messages")?.importance != NotificationManager.IMPORTANCE_NONE
    }

    @Synchronized
    private fun snapshot(): NotificationState {
        enforceAccount()
        check(storageAvailable)
        return value
    }

    private fun accountScope() = notificationAccountScope(GovorunAccountRepository.state.value, BuildConfig.ACCOUNT_API_URL)

    private fun enforceAccount() {
        if (signingOut) return
        val next = value.withAccount(accountScope(), permission(), now())
        if (next != value) { save(next); cancelDisallowed() }
    }

    private fun save(next: NotificationState): Boolean {
        if (!storageAvailable) return false
        return try { storage.write(next); value = next; publish(); true }
        catch (_: Exception) { storageAvailable = false; publish(); false }
    }

    private fun publish() {
        val projection = value.decision("ui", accountScope(), permission(), now(), JSONObject().put("supported", api.configured && transport.available()))
        val notices = projection.getJSONArray("notices")
        mutableUi.value = NotificationUiState(
            storageAvailable && projection.getBoolean("invite"), api.configured, api.configured && transport.available(),
            value.choices, value.prompted, permission(), projection.getBoolean("pending"), storageAvailable,
            (0 until notices.length()).map { UpdateNotice.read(notices.getJSONObject(it)) })
    }

    private fun cancelDisallowed() {
        val manager = NotificationManagerCompat.from(context)
        val projection = value.decision("ui", accountScope(), permission(), now())
        if (!storageAvailable || !projection.getBoolean("updates")) manager.cancel("updates", 8101)
        if (!storageAvailable || !projection.getBoolean("offers")) manager.cancel("offers", 8101)
    }

    private fun randomHex(bytes: Int): String = ByteArray(bytes).also { SecureRandom().nextBytes(it) }
        .joinToString("") { "%02x".format(it.toInt() and 255) }
    private fun now() = System.currentTimeMillis() / 1000
}
