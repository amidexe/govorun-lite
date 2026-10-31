package com.govorun.lite.api.smart

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.productWindow
import com.govorun.lite.network.NetworkPolicy
import com.govorun.lite.stylization.NetworkAvailability
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

/** Process-wide owner of the apps' tasks: they must outlive the service, which Echo binds and unbinds for every call. */
internal object SmartRuntime {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    @Volatile private var tasks: SmartTasks? = null
    private const val JOBS_FILE = "smart-jobs.json"

    fun tasks(context: Context): SmartTasks = tasks ?: synchronized(this) {
        tasks ?: run {
            val app = context.applicationContext
            SmartTasks(AndroidSmartPorts(app, scope), scope, host = AndroidJobHost(app)).also {
                tasks = it
                watchPerson(app, it)
            }
        }
    }

    /** Govorun's process started with tasks kept from before: they are tried as soon as the account is read. */
    fun resume(context: Context) {
        if (java.io.File(context.noBackupFilesDir, JOBS_FILE).exists()) wakeWhenReady(context, person = true)
    }

    /**
     * Wakes the tasks once Govorun's account is read: it comes from storage a moment after the process starts, and until then
     * the person would look signed out and their tasks would wait for a sign-in that is already there.
     */
    fun wakeWhenReady(context: Context, person: Boolean = false, network: Boolean = false): kotlinx.coroutines.Job {
        val app = context.applicationContext
        return scope.launch {
            kotlinx.coroutines.withTimeoutOrNull(ACCOUNT_WAIT_MS) { GovorunAccountRepository.state.first { it.available } }
            tasks(app).wake(person, network)
        }
    }

    /** Reuse the account's connectivity observation; do not create tasks when no app has any. */
    fun networkAvailable(context: Context) {
        if (tasks != null || java.io.File(context.noBackupFilesDir, JOBS_FILE).exists()) {
            wakeWhenReady(context, network = true)
        }
    }

    private const val ACCOUNT_WAIT_MS = 10_000L

    /**
     * The person's steps happen in Govorun: signing in, the terms, Промо, «Офлайн». Their tasks are tried again when the account
     * changes and whenever the person leaves one of Govorun's screens; a step not taken costs only a check on the phone.
     */
    private fun watchPerson(app: Context, tasks: SmartTasks) {
        scope.launch {
            GovorunAccountRepository.state
                .map { Triple(it.signedIn, it.styleAccess.state, it.styleAccess.type) }
                .distinctUntilChanged().drop(1)
                .collect { tasks.wake(person = true) }
        }
        (app as? android.app.Application)?.registerActivityLifecycleCallbacks(object : android.app.Application.ActivityLifecycleCallbacks {
            override fun onActivityPaused(activity: android.app.Activity) = tasks.wake(person = true)
            override fun onActivityCreated(activity: android.app.Activity, state: android.os.Bundle?) {}
            override fun onActivityStarted(activity: android.app.Activity) {}
            override fun onActivityResumed(activity: android.app.Activity) {}
            override fun onActivityStopped(activity: android.app.Activity) {}
            override fun onActivitySaveInstanceState(activity: android.app.Activity, state: android.os.Bundle) {}
            override fun onActivityDestroyed(activity: android.app.Activity) {}
        })
    }

    /** The tasks' file in Govorun's own storage, the wake with a network, and the word to the app. */
    private class AndroidJobHost(private val app: Context) : SmartJobHost {
        private val file = android.util.AtomicFile(java.io.File(app.noBackupFilesDir, JOBS_FILE))
        private val timer = SmartWakeTimer(scope, online = { NetworkAvailability.hasUsableNetwork(app) }) {
            wakeWhenReady(app)
        }

        override fun load(): List<SmartJob> =
            runCatching { SmartJob.parse(file.readFully().toString(Charsets.UTF_8)) }.getOrDefault(emptyList())

        override fun save(jobs: List<SmartJob>) {
            if (jobs.isEmpty()) return file.delete()
            val stream = file.startWrite()
            runCatching { stream.write(SmartJob.list(jobs).toByteArray()); file.finishWrite(stream) }.onFailure { file.failWrite(stream) }
        }

        override fun schedule(atMs: Long?) {
            // JobScheduler may batch a ready job for minutes. While the process lives, use the same deadline directly;
            // the persisted Android job still wakes a process that died. Both enter SmartTasks' single running guard.
            // An already due wake is dispatched by Android. Re-arming a zero-delay timer while that dispatch is
            // pending would recursively request the same job and keep replacing it before Android could start it.
            timer.schedule(atMs?.takeIf { it > System.currentTimeMillis() })
            SmartJobsWake.schedule(app, atMs)
        }

        override fun changed(owner: String, taskId: String) {
            app.sendBroadcast(android.content.Intent(SmartApi.ACTION_TASKS_CHANGED).setPackage(owner)
                .putExtra(SmartApi.EXTRA_TASK_ID, taskId))
        }

        override fun ready() = SmartJobsWake.running()
        override fun waitReason() = if (NetworkPolicy.activeRefusal(app) != null || NetworkAvailability.hasUsableNetwork(app))
            SmartApi.BUSY else SmartState.NO_NETWORK.wire
    }

    fun status(context: Context, authorized: Boolean): SmartStatus {
        if (!authorized) return smartStatus(SmartFacts(authorized = false))
        return smartStatus(SmartFacts(
            authorized = true,
            signedIn = GovorunAccountRepository.state.value.signedIn,
            refusal = NetworkPolicy.activeRefusal(context),
            networkAvailable = NetworkAvailability.hasUsableNetwork(context),
        ))
    }
}

private class AndroidSmartPorts(private val context: Context, private val scope: CoroutineScope) : SmartPorts {
    override suspend fun improve(owner: String, text: String, style: String?): org.json.JSONObject {
        val coordinator = com.govorun.lite.stylization.StylizationCoordinator(context)
        val mode = com.govorun.lite.model.stylization.StylizationInputMode.CONFIRM
        // Govorun's own label for the app's request, from the app it knows, never from what the app sent.
        val product = SmartProducts.of(owner) ?: return org.json.JSONObject().put("code", SmartState.NOT_AUTHORIZED.wire)
        var result = coordinator.process(text, mode, owner, style, product = product)
        // Govorun was still restoring its account after a start (Echo's call may be what started it): the note is refused
        // before anything leaves the phone. A sign-out during the note ends the same way, and the second ask finds no
        // account, so asking once more at once never pays twice.
        if (result.fallbackReason == com.govorun.lite.model.stylization.StylizationFallbackReason.ACCOUNT_STATE_CHANGED) {
            kotlinx.coroutines.delay(1_000)
            result = coordinator.process(text, mode, owner, style, product = product)
        }
        // The note's text handed to Echo is its delivery, as a dictation's insertion: the same rule and the same report,
        // which also saves what the note spent and sends it to the account.
        val outcome = com.govorun.lite.shared.SharedDictationDelivery.prepare(result.text, text, "off", result.usedFallback,
            result.outcomePass != null, result.latencyBucket != "unknown").success.outcome
        val pass = result.outcomePass
        if (pass != null && outcome != null) scope.launch {
            coordinator.reportClientOutcome(pass,
                com.govorun.lite.model.stylization.StylizationClientOutcome.entries.single { it.apiId == outcome }, result.latencyBucket)
        }
        val line = com.govorun.lite.account.GovorunAccountRepository.state.value.productWindow(product)
        return SmartImprove.answer(result, periodSpent = line != null && line.limitTotal > 0 && line.remainingTotal <= 0,
            refusal = com.govorun.lite.network.NetworkPolicy.activeRefusal(context),
            signedIn = com.govorun.lite.account.GovorunAccountRepository.state.value.signedIn)
    }

    override fun digestPrice(text: String): org.json.JSONObject {
        val price = com.govorun.lite.shared.SharedBusiness.evaluate(org.json.JSONObject()
            .put("operation", "styleTextAdmission").put("requestClass", "digest")
            .put("units", org.json.JSONArray(text.map { it.code })))
        val credits = price.getInt("requiredCredits")
        // Echo's own day, the same numbers Govorun's limits card shows for Эхо; Echo says only what share of it итоги take.
        val window = com.govorun.lite.account.GovorunAccountRepository.state.value
            .productWindow(com.govorun.lite.stylization.ProductLabel.ECHO)
        return org.json.JSONObject().put("characters", price.getInt("characters")).put("credits", credits)
            .put("allowed", price.getBoolean("allowed")).put("reason", price.opt("reason") ?: org.json.JSONObject.NULL)
            .put("confirm", price.getBoolean("confirm"))
            .put("minChars", price.getInt("minimumCharacters")).put("maxChars", price.getInt("maximumCharacters"))
            .put("limitToday", window?.limitToday ?: org.json.JSONObject.NULL)
            .put("remainingToday", window?.remainingToday ?: org.json.JSONObject.NULL)
    }
}
