package com.govorun.lite.api.smart

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Handler
import android.os.Looper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** One Android job grants background execution before an app's task starts its network work. The same job keeps
 * Govorun runnable until the attempts finish; future retries stay persisted, without holding a service while waiting. */
class SmartJobsWake : JobService() {
    private var waiting: Job? = null

    override fun onStartJob(params: JobParameters): Boolean {
        if (params.network == null) {
            // A deadline can arrive just as the network disappears. No attempt or I/O: wait for the network constraint.
            active = false
            granted = false
            applySchedule(this, nextAt ?: System.currentTimeMillis())
            return false
        }
        active = true
        val tasks = SmartRuntime.tasks(this)
        waiting = CoroutineScope(Dispatchers.IO).launch {
            // Closing an idle TLS connection can write its close-notify. Keep it off Android's main thread too.
            com.govorun.lite.account.AccountHttpTransport.beginBackgroundWork()
            ensureActive()
            granted = true
            val woken = SmartRuntime.wakeWhenReady(this@SmartJobsWake, network = true)
            woken.join()
            while (true) {
                while (tasks.running()) delay(200)
                val finished = withContext(Dispatchers.Main) {
                    // A Binder call may have started another attempt since the worker observed the last one finish.
                    if (tasks.running()) false else {
                        active = false
                        granted = false
                        jobFinished(params, false)
                        applySchedule(this@SmartJobsWake, nextAt)
                        true
                    }
                }
                if (finished) return@launch
            }
        }
        return true
    }

    override fun onStopJob(params: JobParameters): Boolean {
        active = false
        granted = false
        waiting?.cancel()
        return true
    }

    companion object {
        private const val ID = 8105
        private const val WINDOW_MS = 60_000L
        private val main = Handler(Looper.getMainLooper())
        @Volatile private var active = false
        @Volatile private var granted = false
        // Main-thread only: callbacks and schedule changes cannot replace a job while its work is still running.
        private var nextAt: Long? = null
        private var submittedAt: Long? = null

        private fun networkPresent(context: Context): Boolean {
            val manager = context.getSystemService(ConnectivityManager::class.java) ?: return false
            return manager.activeNetwork?.let(manager::getNetworkCapabilities)
                ?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true
        }

        fun running(): Boolean = granted

        fun schedule(context: Context, atMs: Long?) {
            val app = context.applicationContext
            main.post {
                nextAt = atMs
                if (!active) applySchedule(app, atMs)
            }
        }

        private fun applySchedule(context: Context, atMs: Long?) {
            val scheduler = context.getSystemService(JobScheduler::class.java) ?: return
            if (atMs == null) {
                submittedAt = null
                return scheduler.cancel(ID)
            }
            val delay = (atMs - System.currentTimeMillis()).coerceAtLeast(0)
            fun ordinary(): JobInfo = JobInfo.Builder(ID, ComponentName(context, SmartJobsWake::class.java))
                .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY).setPersisted(true)
                .setMinimumLatency(delay)
                .apply {
                    // A ready job with no latest time can be batched for many minutes. A full minute's window is
                    // allowed by Android 15+; offline jobs have no deadline and never repeatedly wake the phone.
                    if (networkPresent(context)) setOverrideDeadline(delay + WINDOW_MS)
                }.build()
            try {
                val immediate = delay == 0L && networkPresent(context)
                if (submittedAt == atMs && scheduler.getPendingJob(ID)?.isExpedited == immediate) return
                val job = if (immediate) JobInfo.Builder(ID, ComponentName(context, SmartJobsWake::class.java))
                    .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY).setPersisted(true).setExpedited(true).build() else ordinary()
                var result = scheduler.schedule(job)
                if (result != JobScheduler.RESULT_SUCCESS && immediate) result = scheduler.schedule(ordinary())
                if (result == JobScheduler.RESULT_SUCCESS) submittedAt = atMs
            } catch (error: RuntimeException) {
                // Fixed class only, never a task, its text, an account, or an exception message.
                android.util.Log.w("GovorunSmart", "event=job_schedule_failed error=${error.javaClass.simpleName}")
            }
        }
    }
}
