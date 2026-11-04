package com.govorun.lite.api.smart

import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import com.govorun.api.smart.v1.ISmartCallback
import com.govorun.api.smart.v1.ISmartService
import com.govorun.lite.api.CallerIdentity
import com.govorun.lite.api.CallerIdentityResolver
import com.govorun.lite.api.RecognitionApi
import com.govorun.lite.api.RecognitionAuthorizationStore
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.flow.first

/**
 * Govorun's bridge for Говорун Эхо, and for no other app ([EchoApp]): status, Govorun's screens and «Улучшить через Говорун».
 * The caller's package must match its Binder UID, be Echo by its signature and have the access to recognition the person gave
 * it. getStatus is answered from memory; an improvement leaves only from runTask, through Govorun's own text improvement.
 */
class SmartService : Service() {
    @Volatile private var accountRead = false

    private lateinit var identities: CallerIdentityResolver
    private lateinit var authorizations: RecognitionAuthorizationStore

    override fun onCreate() {
        super.onCreate()
        identities = CallerIdentityResolver(this)
        authorizations = RecognitionAuthorizationStore(this)
    }

    override fun onBind(intent: Intent?): IBinder? = binder.takeIf { intent?.action == SmartApi.SERVICE_ACTION }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Bind-only, like the recognition API: a started service would outlive its clients.
        stopSelf(startId)
        return START_NOT_STICKY
    }

    private val binder = object : ISmartService.Stub() {
        override fun getProtocolVersion(): Int = SmartApi.PROTOCOL_VERSION

        override fun getStatus(clientPackage: String?): String {
            awaitAccount()
            val identity = caller(clientPackage)
            val allowed = identity?.let(::hasSmart) == true
            // The caller's own volume-key permission and Govorun's switch: nothing about the account.
            val volumeKey = org.json.JSONObject()
                .put("granted", identity?.let { authorizations.hasScope(it, com.govorun.lite.api.RecognitionApi.SCOPE_VOLUME_KEY) } == true)
                .put("enabled", com.govorun.lite.util.VolumeKeyPrefs.isEnabled(this@SmartService))
            // One text-task capability; operation names and response shapes belong to the server and Echo.
            return org.json.JSONObject(SmartRuntime.status(this@SmartService, allowed).json()).put("volumeKey", volumeKey).put("textTasks", allowed)
                .apply {
                    if (allowed) {
                        put("styles", SmartResult.styles(this@SmartService))
                        put("capabilities", org.json.JSONArray().put(SmartApi.CAPABILITY_TEXT_TASKS).put(ClientNetworkPolicy.CAPABILITY))
                        // The person once pressed a smart function in the app: its own tasks may go (protocol 2).
                        put("smart", identity?.let { authorizations.hasScope(it, RecognitionApi.SCOPE_SMART_FUNCTIONS) } == true)
                    }
                }.toString()
        }

        override fun createAction(clientPackage: String?, action: String?): PendingIntent {
            val identity = caller(clientPackage) ?: throw SecurityException("Calling package does not match Binder UID")
            if (!EchoApp.matches(identity)) throw SecurityException("The smart functions are Говорун Эхо's only")
            return SmartActions.create(this@SmartService, identity, action.orEmpty(), authorizations)
        }

        override fun runTask(
            clientPackage: String?,
            taskId: String?,
            kind: String?,
            inputJson: String?,
            callback: ISmartCallback?,
        ) {
            val id = taskId.orEmpty()
            val tasks = SmartRuntime.tasks(this@SmartService)
            val identity = authorized(clientPackage)
            // Echo closing does not stop the task: Govorun keeps it until Echo collects the answer (SmartTasks).
            val reply = callback?.let(::CallbackReply) ?: return
            if (identity == null) return reply.error(SmartRefusal.of(SmartState.NOT_AUTHORIZED))
            if (kind == ClientNetworkPolicy.KIND) {
                return try {
                    SmartRuntime.networkPolicy(this@SmartService).set(identity.packageName, inputJson.orEmpty())
                    reply.result("{}")
                } catch (_: Exception) { reply.error(SmartRefusal.of(SmartState.UNAVAILABLE)) }
            }
            if (kind == SmartApi.COLLECT) {
                // Receiving paid work is local, also while offline or with automatic functions disabled.
                // In particular it must never approve the app's smart scope again.
                tasks.collect(identity.packageName, id, reply)
                return
            }
            if (kind !in setOf(SmartApi.TEXT_TASK, SmartApi.TASK_PRICE)) return reply.error(SmartRefusal.of(SmartApi.REJECTED_INPUT))
            // The person's own press is their yes to the app's smart functions; the app's own task needs that yes first.
            val asked = runCatching { org.json.JSONObject(inputJson.orEmpty()).get("asked") as Boolean }.getOrNull()
            if (kind != SmartApi.TASK_PRICE) {
                // Every task says whether the person pressed it.
                if (asked == null) return reply.error(SmartRefusal.of(SmartApi.REJECTED_INPUT))
                val enabled = authorizations.hasScope(identity, RecognitionApi.SCOPE_SMART_FUNCTIONS)
                if (asked && !enabled) authorizations.approve(identity, RecognitionApi.SCOPE_SMART_FUNCTIONS)
                if (!asked && !enabled) return reply.error(SmartRefusal.of(SmartApi.SMART_OFF))
            }
            tasks.run(identity.packageName, id, kind.orEmpty(), inputJson.orEmpty(), reply)
        }

        override fun cancelTask(clientPackage: String?, taskId: String?) {
            val identity = caller(clientPackage) ?: return
            SmartRuntime.tasks(this@SmartService).cancel(identity.packageName, taskId.orEmpty())
        }

        override fun getTasks(clientPackage: String?): String {
            val identity = authorized(clientPackage) ?: return "[]"
            return SmartRuntime.tasks(this@SmartService).list(identity.packageName)
        }
    }

    /**
     * Echo's call may be what woke Govorun: its account is read from storage a moment after the process starts, and until then it
     * would look signed out. The first call waits for it, a few seconds at most, once per process.
     */
    private fun awaitAccount() {
        if (accountRead || com.govorun.lite.account.GovorunAccountRepository.state.value.available) return
        kotlinx.coroutines.runBlocking {
            kotlinx.coroutines.withTimeoutOrNull(ACCOUNT_WAIT_MS) { com.govorun.lite.account.GovorunAccountRepository.state.first { it.available } }
        }
        accountRead = true
    }

    private fun caller(clientPackage: String?): CallerIdentity? =
        clientPackage?.let { identities.resolve(Binder.getCallingUid(), it) }

    private fun hasSmart(identity: CallerIdentity) =
        EchoApp.matches(identity) && authorizations.hasScope(identity, RecognitionApi.SCOPE_LOCAL_RECOGNITION)

    private fun authorized(clientPackage: String?): CallerIdentity? =
        caller(clientPackage)?.takeIf { EchoApp.matches(it) && authorizations.isAuthorized(it) }

    /** One callback per call; a client that is gone simply misses it. */
    private class CallbackReply(private val callback: ISmartCallback) : SmartReply {
        private val done = AtomicBoolean(false)

        override fun result(json: String) = finish { callback.onResult(json) }

        override fun error(refusal: SmartRefusal) = finish { callback.onError(refusal.code, refusal.message, refusal.retryAfterMs) }

        private fun finish(send: () -> Unit) {
            if (done.compareAndSet(false, true)) runCatching(send)
        }
    }

    private companion object {
        const val ACCOUNT_WAIT_MS = 3_000L
    }
}
