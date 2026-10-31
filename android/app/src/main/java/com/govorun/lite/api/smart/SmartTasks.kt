package com.govorun.lite.api.smart

import com.govorun.lite.account.AccountSessionChangedException
import com.govorun.lite.account.SecureStorageUnavailableException
import com.govorun.lite.shared.SharedBusinessException
import com.govorun.lite.stylization.EchoNoteMark
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import org.json.JSONArray
import org.json.JSONObject
import java.security.MessageDigest

/** The effects Echo may ask for; the Android port reaches Govorun's own text improvement. */
internal interface SmartPorts {
    /** The text through Govorun's own text improvement for app [owner]: `{"text"}` or `{"code"}` ([SmartImprove]). */
    suspend fun improve(owner: String, text: String, style: String?): JSONObject
    /** What итоги of [text] cost, from the core's rule and Echo's day: nothing leaves the phone. */
    fun digestPrice(text: String): JSONObject
}

/** Exactly one answer per call; the service turns it into the client's callback. */
internal interface SmartReply {
    fun result(json: String)
    fun error(refusal: SmartRefusal)
}

/** Where Govorun keeps the tasks between tries, and what it asks of Android for them. */
internal interface SmartJobHost {
    fun load(): List<SmartJob> = emptyList()
    fun save(jobs: List<SmartJob>) {}
    /** Wake Govorun with a network at [atMs] or later; null: no task waits on a time. */
    fun schedule(atMs: Long?) {}
    /** Tell the app that its task has its answer or waits for something else. */
    fun changed(owner: String, taskId: String) {}
    /** A try started; a host may observe its lifetime. */
    fun started() {}
    /** Android grants background execution before an attempt may use its network. Tests need no platform grant. */
    fun ready(): Boolean = true
    fun waitReason(): String = SmartApi.BUSY
}

/**
 * Echo's «Улучшить через Говорун» and итоги as tasks Govorun keeps (protocol 2). The app hands a task in once and hears its
 * answer, «принято» (`queued`, with Govorun's words for why it waits) or a final refusal. A waiting task is tried again when its
 * cause is gone: a network, the server back, the next UTC day, or the person's own step in Govorun, which only a task the person
 * pressed waits for. A task outlives Echo and Govorun's process, is paid once, and its text is dropped once it has its answer.
 */
internal class SmartTasks(
    private val ports: SmartPorts,
    private val scope: CoroutineScope,
    private val clock: () -> Long = System::currentTimeMillis,
    private val host: SmartJobHost = object : SmartJobHost {},
) {
    private class Running(val hash: String, val replies: MutableList<SmartReply>) {
        lateinit var job: Job
        @Volatile var cancelled = false
    }

    private val lock = Any()
    private val running = mutableMapOf<Pair<String, String>, Running>()
    private val jobs = linkedMapOf<Pair<String, String>, SmartJob>()

    init {
        host.load().forEach { jobs[it.owner to it.taskId] = it }
    }

    fun run(owner: String, taskId: String, kind: String, inputJson: String, reply: SmartReply) {
        if (kind != SmartApi.IMPROVE && kind != SmartApi.DIGEST && kind != SmartApi.DIGEST_PRICE) {
            return reply.error(SmartRefusal.of(SmartState.UNAVAILABLE))
        }
        if (inputJson.toByteArray(Charsets.UTF_8).size > SmartApi.MAX_INPUT_BYTES) {
            return reply.error(SmartRefusal.of(SmartImprove.TOO_LONG))
        }
        val input = runCatching { JSONObject(inputJson) }.getOrNull()
        val text = input?.optString("text").orEmpty()
        if (input == null || text.isBlank() || !validId(taskId)) {
            return reply.error(SmartRefusal.of(SmartApi.REJECTED_INPUT))
        }
        if (kind == SmartApi.DIGEST_PRICE) return reply.result(ports.digestPrice(text).toString())
        // A note sent with no style is still Echo's: «Заметка», the first of Govorun's styles for Echo.
        val style = if (kind == SmartApi.DIGEST) EchoNoteMark.DIGEST_STYLE
            else input.optString("style").takeIf { it.isNotBlank() } ?: EchoNoteMark.NOTE_STYLE
        // Every task says whether the person pressed it.
        val asked = input.opt("asked") as? Boolean ?: return reply.error(SmartRefusal.of(SmartApi.REJECTED_INPUT))
        val job = SmartJob(owner, taskId, kind, hash(kind, style, text), JSONObject().put("text", text).put("style", style).toString(), asked)
        val now = clock()
        val key = owner to taskId
        var answer: String? = null
        var queued: SmartJob? = null
        synchronized(lock) {
            prune(now)
            val current = running[key]
            val known = jobs[key]
            when {
                current != null && current.hash == job.hash -> { current.replies += reply; return }
                current != null -> return reply.error(SmartRefusal.of(SmartApi.REJECTED_INPUT))
                // A collected text is the app's for good; a failure is told again unless the person presses anew.
                known != null && known.hash == job.hash && known.answer != null && !(asked && known.failed) -> {
                    answer = known.answer
                    if (known.failed && JSONObject(checkNotNull(known.answer)).optString("code") in PERSON) jobs.remove(key)
                    else jobs[key] = known.copy(atMs = minOf(known.atMs, now + REMEMBER_MILLIS))
                    save()
                }
                else -> {
                    // The same task waiting is tried at once (the app asks again: its cause may be gone); a pressed one stays pressed.
                    val next = if (known != null && known.hash == job.hash) known.copy(asked = known.asked || asked) else job
                    when {
                        jobs.values.count { it.owner == owner && it.waiting } >= MAX_WAITING_PER_APP && known == null ->
                            return reply.error(SmartRefusal(SmartApi.SERVER_ERROR,
                                "Достигнут лимит ожидающих задач. Дождитесь завершения предыдущих", BUSY_WAIT_MILLIS))
                        !host.ready() -> {
                            queued = next.copy(wait = host.waitReason(), atMs = now)
                            jobs[key] = queued!!
                            save()
                        }
                        // One итоги at a time and a few tries per app: the rest waits its turn here.
                        running.keys.count { it.first == owner } >= SmartApi.MAX_TASKS_PER_APP ||
                            kind == SmartApi.DIGEST && running.keys.any { it.first == owner && jobs[it]?.kind == SmartApi.DIGEST } -> {
                            queued = next.copy(wait = SmartApi.BUSY, atMs = now + TURN_MILLIS)
                            jobs[key] = queued!!
                            save()
                        }
                        else -> start(key, next, mutableListOf(reply))
                    }
                }
            }
        }
        answer?.let { deliver(reply, it) }
        queued?.let {
            reply.error(SmartRefusal(SmartApi.QUEUED, SmartJob.waitWords(checkNotNull(it.wait))))
            host.schedule(nextTimed())
        }
    }

    /** Tries every waiting task whose time came; [person]: the person took a step in Govorun, so their tasks are tried too. */
    fun wake(person: Boolean = false, network: Boolean = false) {
        val now = clock()
        synchronized(lock) {
            prune(now)
            if (!host.ready()) {
                jobs.replaceAll { _, job ->
                    if (job.waiting && (person && job.atMs == SmartJob.PERSON || network && job.wait == SmartState.NO_NETWORK.wire)) {
                        job.copy(atMs = now)
                    } else job
                }
                save()
                host.schedule(nextTimed())
                return
            }
            val digests = running.keys.filter { jobs[it]?.kind == SmartApi.DIGEST }.map { it.first }.toMutableSet()
            jobs.entries.filter { (key, job) ->
                key !in running && job.waiting && job.input != null &&
                    (job.atMs <= now || person && job.atMs == SmartJob.PERSON ||
                        network && job.wait == SmartState.NO_NETWORK.wire)
            }.forEach { (key, job) ->
                if (running.keys.count { it.first == job.owner } >= SmartApi.MAX_TASKS_PER_APP) return@forEach
                if (job.kind == SmartApi.DIGEST && !digests.add(job.owner)) return@forEach
                start(key, job, mutableListOf())
            }
        }
        host.schedule(nextTimed())
    }

    /** The app's tasks as its list shows them. */
    fun list(owner: String): String = synchronized(lock) {
        prune(clock())
        JSONArray().apply { jobs.values.filter { it.owner == owner }.forEach { put(it.listed()) } }.toString()
    }

    /** Whether any task is being tried now. */
    fun running(): Boolean = synchronized(lock) { running.isNotEmpty() }

    /** The app gave the task up: it stops and is forgotten. */
    fun cancel(owner: String, taskId: String) {
        val task = synchronized(lock) {
            jobs.remove(owner to taskId)
            save()
            running[owner to taskId]?.also { it.cancelled = true }
        }
        task?.job?.cancel()
    }

    fun cancelAll(owner: String) {
        val tasks = synchronized(lock) {
            jobs.keys.removeAll { it.first == owner }
            save()
            running.filterKeys { it.first == owner }.values.onEach { it.cancelled = true }.toList()
        }
        tasks.forEach { it.job.cancel() }
    }

    /** The person disabled automatic functions: already waiting automatic tasks lose that permission too. */
    fun cancelAutomatic(owner: String) {
        val ids = synchronized(lock) { jobs.values.filter { it.owner == owner && !it.asked && it.waiting }.map { it.taskId } }
        ids.forEach { cancel(owner, it); host.changed(owner, it) }
        host.schedule(nextTimed())
    }

    /** Called under the lock: registers the try before it can finish. */
    private fun start(key: Pair<String, String>, job: SmartJob, replies: MutableList<SmartReply>) {
        val task = Running(job.hash, replies)
        jobs[key] = job.copy(wait = null)
        save()
        task.job = scope.launch(start = CoroutineStart.LAZY) {
            val outcome = try {
                withTimeout(IMPROVE_MILLIS) { attempt(job) }
            } catch (cancelled: CancellationException) {
                Result.failure(Refused(SmartRefusal.of(
                    if (cancelled is kotlinx.coroutines.TimeoutCancellationException) SmartApi.TIMEOUT else SmartApi.CANCELLED)))
            } catch (error: Exception) {
                Result.failure(Refused(failure(error)))
            }
            settle(key, job, task, outcome)
        }
        running[key] = task
        task.job.start()
        host.started()
    }

    private suspend fun attempt(job: SmartJob): Result<String> {
        val input = JSONObject(checkNotNull(job.input))
        val digest = job.kind == SmartApi.DIGEST
        val answer = ports.improve(job.owner, input.getString("text"), input.getString("style"))
        return when {
            !answer.has("text") -> Result.failure(Refused(SmartRefusal.of(answer.optString("code", SmartApi.SERVER_ERROR))))
            !digest -> Result.success(JSONObject().put("text", answer.getString("text")).toString())
            // Итоги come back in the server's shape (a name, main points, lists); anything else (the note itself) is no answer,
            // and it was paid, so it is tried once more only.
            else -> runCatching { JSONObject(answer.getString("text")).also { it.getString("nazvanie"); it.getJSONArray("glavnoe") } }.fold(
                { Result.success(JSONObject().put("digest", it).toString()) },
                { Result.failure(Refused(SmartRefusal.of(SmartApi.MODEL_ANSWER))) })
        }
    }

    private fun settle(key: Pair<String, String>, job: SmartJob, task: Running, outcome: Result<String>) {
        val now = clock()
        val (next, replies) = synchronized(lock) {
            if (running[key] === task) running.remove(key)
            if (task.cancelled) null to task.replies.toList() else settled(key, job, task, outcome, now)
        }
        if (next == null) return replies.forEach { it.error(SmartRefusal.of(SmartApi.CANCELLED)) }
        if (next.answer != null) replies.forEach { deliver(it, next.answer) }
        else replies.forEach { it.error(SmartRefusal(SmartApi.QUEUED, SmartJob.waitWords(checkNotNull(next.wait)))) }
        // A try nobody waited on: the app hears that its task changed.
        if (replies.isEmpty()) host.changed(job.owner, job.taskId)
        if (job.kind == SmartApi.DIGEST) wake() else host.schedule(nextTimed())
    }

    /** Called under the lock: what the try's [outcome] leaves of the task, kept for the app. */
    private fun settled(key: Pair<String, String>, job: SmartJob, task: Running, outcome: Result<String>, now: Long):
        Pair<SmartJob, List<SmartReply>> {
            val next = outcome.fold(
                { json -> job.copy(input = null, wait = null, answer = json,
                    atMs = now + if (task.replies.isEmpty()) FORGET_MILLIS else REMEMBER_MILLIS) },
                { error ->
                    val code = (error as? Refused)?.refusal?.code ?: SmartApi.SERVER_ERROR
                    // A failure is told for a short while only, so a new press tries again.
                    retryAt(code, job.asked, job.tries, now)?.let { job.copy(wait = code, atMs = it, tries = job.tries + 1) }
                        ?: job.copy(input = null, wait = null, answer = JSONObject().put("code", code).toString(), atMs = now + REMEMBER_MILLIS)
                })
            // The person's own step for a task they did not press: told at once and not kept, so it never outlives the step.
            if (task.replies.isNotEmpty() && next.failed && JSONObject(checkNotNull(next.answer)).getString("code") in PERSON) jobs.remove(key)
            else jobs[key] = next
            save()
            return next to task.replies.toList()
        }

    private fun deliver(reply: SmartReply, answer: String) {
        val value = JSONObject(answer)
        if (value.has("code")) reply.error(SmartRefusal.of(value.getString("code"))) else reply.result(answer)
    }

    /** Called under the lock: answers nobody collected in time and answers already collected are forgotten. */
    private fun prune(now: Long) {
        if (jobs.values.removeAll { !it.waiting && it.atMs <= now }) save()
    }

    private fun save() = host.save(jobs.values.toList())

    /** The earliest timed try, null when nothing waits on a time. */
    private fun nextTimed(): Long? = synchronized(lock) {
        jobs.filter { (key, job) ->
            job.waiting && job.atMs != SmartJob.PERSON && key !in running &&
                running.keys.count { it.first == job.owner } < SmartApi.MAX_TASKS_PER_APP &&
                (job.kind != SmartApi.DIGEST || running.keys.none { it.first == job.owner && jobs[it]?.kind == SmartApi.DIGEST })
        }.values.minOfOrNull { it.atMs }
    }

    private fun failure(error: Exception): SmartRefusal = when (error) {
        is AccountSessionChangedException -> SmartRefusal.of(SmartApi.CANCELLED)
        is SecureStorageUnavailableException -> SmartRefusal.of(SmartState.UNAVAILABLE)
        is SharedBusinessException -> when (error.code) {
            "account_signed_out" -> SmartRefusal.of(SmartState.NOT_SIGNED_IN)
            "account_invalid_request" -> SmartRefusal.of(SmartApi.REJECTED_INPUT)
            else -> SmartRefusal.of(SmartApi.SERVER_ERROR)
        }
        else -> SmartRefusal.of(SmartApi.SERVER_ERROR)
    }

    private class Refused(val refusal: SmartRefusal) : Exception(refusal.code)

    companion object {
        /** Govorun's core gives a note 130 s; a try ends after 140 s. */
        const val IMPROVE_MILLIS = 140_000L
        /** How long an answer the app collected stays here for the app to ask again. */
        const val REMEMBER_MILLIS = 15 * 60_000L
        /** How long an answer waits for the app to collect it. */
        const val FORGET_MILLIS = 7 * 24 * 3_600_000L
        /** Too many tasks of one app waiting: try again shortly. */
        const val BUSY_WAIT_MILLIS = 1_000L
        /** A task waiting its turn behind another итоги or the app's other tries. */
        const val TURN_MILLIS = 30_000L
        const val MAX_WAITING_PER_APP = 50
        /** Server down, busy, slow or a moment of Govorun's account changing: again soon, then more rarely, never given up. */
        val SERVER_BACKOFF_MS = longArrayOf(30_000L, 120_000L, 600_000L, 1_800_000L)
        /** No network: Android wakes Govorun only with one, so these only space the tries when the network is there but fails. */
        val NETWORK_BACKOFF_MS = longArrayOf(30_000L, 60_000L, 120_000L, 300_000L)
        private const val DAY_MS = 86_400_000L

        private val SERVER = setOf(SmartState.UNAVAILABLE.wire, SmartApi.SERVER_ERROR, SmartApi.TIMEOUT, SmartApi.BUSY, SmartApi.CANCELLED)
        private val PERSON = setOf(SmartState.NOT_SIGNED_IN.wire, SmartState.NEEDS_ACCESS.wire, SmartState.OFFLINE.wire,
            SmartState.ROAMING.wire, SmartImprove.STYLIZATION_TERMS, SmartImprove.LIMIT_PERIOD)

        /**
         * When a task that ended with [code] is tried again; null when it never would get another answer (a text too short or
         * too long, Echo's improvement switched off by the person), or when it waits for the person but was not pressed.
         */
        fun retryAt(code: String, asked: Boolean, tries: Int, now: Long): Long? = when (code) {
            SmartState.NO_NETWORK.wire -> now + NETWORK_BACKOFF_MS[tries.coerceAtMost(NETWORK_BACKOFF_MS.size - 1)]
            // Echo's day is a UTC day, as the account counts it; spread over the first minutes of the next one.
            SmartState.LIMIT_TODAY.wire -> (now / DAY_MS + 1) * DAY_MS + (now % 600_000L)
            in SERVER -> now + SERVER_BACKOFF_MS[tries.coerceAtMost(SERVER_BACKOFF_MS.size - 1)]
            in PERSON -> if (asked) SmartJob.PERSON else null
            // A paid answer that is no итоги: once more, then it ends.
            SmartApi.MODEL_ANSWER -> if (tries == 0) now + SERVER_BACKOFF_MS[0] else null
            else -> null
        }

        fun hash(kind: String, style: String, text: String): String =
            MessageDigest.getInstance("SHA-256").digest("$kind\n$style\n$text".toByteArray()).joinToString("") { "%02x".format(it) }

        fun validId(value: String): Boolean =
            value.isNotEmpty() && value.length <= SmartApi.MAX_TASK_ID && value.none(Char::isISOControl)
    }
}
