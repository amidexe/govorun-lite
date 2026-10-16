package com.govorun.lite.account

import android.content.Context
import java.time.LocalDate
import kotlin.random.Random
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONObject

internal class AccountSyncCoordinator(
    context: Context,
    private val scope: CoroutineScope,
    private val secure: AccountSessionReader,
    private val stats: StatsSyncStore,
    private val preferences: PortablePreferenceSyncStore,
    private val content: AccountContentSyncCoordinator,
    private val enrollment: AccountSyncEnrollmentStore,
    private val api: AccountApiClient,
    private val onSynchronized: (String) -> Unit = {},
) {
    private val appContext = context.applicationContext
    private val mutex = Mutex()
    private var job: Job? = null
    private var consecutiveFailures = 0
    private var failureScope: GovorunSession? = null
    private var paused = false

    @Synchronized
    fun request(delayMillis: Long? = null) {
        val expected = secure.session() ?: return
        if (failureScope?.let { sameLogicalAccount(it, expected) } != true || delayMillis == 0L) {
            failureScope = expected
            consecutiveFailures = 0
            paused = false
        }
        if (job?.isActive == true || paused) return
        job = scope.launch {
            var nextDelay: Long? = null
            try {
                val initialDelay = delayMillis ?: SynchronizationRules.evaluate(JSONObject()
                    .put("kind", "schedule").put("failures", 0).put("succeeded", true)
                    .put("pending", true).put("jitter", 0.0)).getLong("delayMillis")
                delay(initialDelay)
                val succeeded = synchronizeNow(expected)
                val hasPending = secure.hasSession(expected) &&
                    expected.accountId?.let(::hasPending) == true
                synchronized(this@AccountSyncCoordinator) {
                    val decision = SynchronizationRules.evaluate(JSONObject().put("kind", "schedule")
                        .put("failures", consecutiveFailures).put("succeeded", succeeded)
                        .put("pending", hasPending).put("jitter", Random.nextDouble()))
                    consecutiveFailures = decision.getInt("failures")
                    paused = hasPending && !succeeded && decision.isNull("delayMillis")
                    if (!decision.isNull("delayMillis")) nextDelay = decision.getLong("delayMillis")
                }
            } finally {
                synchronized(this@AccountSyncCoordinator) { job = null }
            }
            if (secure.hasSession(expected)) nextDelay?.let(::request)
        }
    }

    suspend fun synchronizeNow(expected: GovorunSession? = secure.session()): Boolean = syncAttempt {
        mutex.withLock { synchronizeCurrent(expected) }
    }.also { if (it) synchronized(this) { consecutiveFailures = 0; paused = false } }

    private suspend fun synchronizeCurrent(expected: GovorunSession?): Boolean {
        val initial = expected ?: return true
        val accountId = initial.accountId ?: return true
        var session = liveSession(initial, accountId)
        if (!enrollment.isEnabled(accountId)) return true
        val identity = SynchronizationRules.scope(accountId)
        var flow = SynchronizationRules.evaluate(JSONObject().put("kind", "start").put("scope", identity))
        while (true) {
            session = liveSession(session, accountId)
            val action = flow.getString("action")
            if (action == "complete") return flow.getBoolean("succeeded")
            if (action == "cancelled") return false
            val succeeded = syncAttempt {
                when (action) {
                    "content" -> content.synchronize(session, accountId)
                    "preferences" -> synchronizePreferences(session, accountId)
                    "statistics" -> synchronizeStatistics(session, accountId)
                    else -> error("Invalid synchronization action")
                }
                true
            }
            session = liveSession(session, accountId)
            flow = SynchronizationRules.evaluate(JSONObject().put("kind", "advance")
                .put("flow", flow.getJSONObject("flow")).put("scope", identity)
                .put("stage", action).put("succeeded", succeeded))
        }
    }

    private suspend fun synchronizeStatistics(initial: GovorunSession, accountId: String) {
        var session = liveSession(initial, accountId)
        var decision = stats.step(JSONObject().put("kind", "prepare")
            .put("account", accountId).put("today", LocalDate.now().toString()))
        if (decision.getString("action") == "delete") {
            authorized(session, accountId) { token -> api.deleteStats(token) }
            session = liveSession(session, accountId)
            stats.step(JSONObject().put("kind", "deleteAccepted").put("account", accountId)
                .put("epoch", decision.getLong("epoch")))
            decision = stats.step(JSONObject().put("kind", "prepare")
                .put("account", accountId).put("today", LocalDate.now().toString()))
        }
        check(decision.getString("action") == "sync")
        val response = authorized(session, accountId) { token ->
            api.statisticsDocument(token, decision.getJSONObject("request"))
        }
        session = liveSession(session, accountId)
        stats.step(JSONObject().put("kind", "accept").put("account", accountId)
            .put("receipt", decision.getJSONObject("receipt")).put("response", response))
        onSynchronized(accountId)
    }

    private suspend fun synchronizePreferences(session: GovorunSession, accountId: String) {
        var flow: JSONObject? = null
        var event = JSONObject().put("kind", "start")
        while (true) {
            liveSession(session, accountId)
            val decision = preferences.step(accountId, flow, event)
            flow = decision.optJSONObject("flow")
            when (decision.getString("action")) {
                "complete" -> return
                "failed" -> error("Preference synchronization deferred")
                "read", "patch" -> {
                    event = try {
                        val document = authorized(session, accountId) { token ->
                            if (decision.getString("action") == "read") api.readPreferenceDocument(token)
                            else api.writePreferenceDocument(token, decision.getJSONObject("patch"))
                        }
                        JSONObject().put("kind", "document").put("document", document)
                    } catch (error: Exception) {
                        if (error is CancellationException) throw error
                        JSONObject().put("kind", "failed").put("status",
                            (error as? AccountHttpException)?.statusCode ?: JSONObject.NULL)
                    }
                }
                else -> error("Invalid preference decision")
            }
        }
    }

    private suspend fun <T> authorized(
        session: GovorunSession,
        accountId: String,
        call: suspend (String) -> T,
    ): T {
        val attempted = liveSession(session, accountId)
        val result = call(attempted.accessToken)
        liveSession(session, accountId)
        return result
    }

    private fun liveSession(fallback: GovorunSession, accountId: String): GovorunSession {
        val live = secure.session() ?: throw AccountSessionChangedException()
        if (live.accountId != accountId || !sameLogicalAccount(fallback, live)) throw AccountSessionChangedException()
        return live
    }

    private fun hasPending(accountId: String): Boolean =
        enrollment.isEnabled(accountId) && (
            stats.hasPending(accountId) || stats.isResetPending(accountId) ||
            preferences.hasPending(accountId) || content.hasPending(accountId)
            )

    fun isEnabled(accountId: String): Boolean = enrollment.isEnabled(accountId)


}

private suspend fun syncAttempt(work: suspend () -> Boolean): Boolean = try {
    work()
} catch (error: Exception) {
    if (error is CancellationException) throw error
    false
}
