package com.govorun.lite.notifications

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class NotificationSyncJob : JobService() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var work: Job? = null

    override fun onStartJob(params: JobParameters): Boolean {
        work = scope.launch {
            val success = NotificationRepository.reconcile()
            jobFinished(params, !success)
        }
        return true
    }

    override fun onStopJob(params: JobParameters): Boolean { work?.cancel(); return true }
    override fun onDestroy() { scope.cancel(); super.onDestroy() }

    companion object {
        private const val ID = 8102
        private const val RETRY_ID = 8103
        fun schedule(context: Context, pending: Boolean) {
            val scheduler = context.getSystemService(JobScheduler::class.java)
            if (scheduler.getPendingJob(ID) == null) {
                scheduler.schedule(builder(context, ID).setPeriodic(24 * 60 * 60 * 1000L).build())
            }
            if (pending) retry(context)
        }

        fun retry(context: Context) {
            val scheduler = context.getSystemService(JobScheduler::class.java)
            if (scheduler.getPendingJob(RETRY_ID) == null) scheduler.schedule(builder(context, RETRY_ID).build())
        }

        private fun builder(context: Context, id: Int) = JobInfo.Builder(id, ComponentName(context, NotificationSyncJob::class.java))
            .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY).setPersisted(true)
            .setBackoffCriteria(15 * 60 * 1000L, JobInfo.BACKOFF_POLICY_EXPONENTIAL)

        fun cancel(context: Context) {
            context.getSystemService(JobScheduler::class.java).apply { cancel(ID); cancel(RETRY_ID) }
        }
    }
}
