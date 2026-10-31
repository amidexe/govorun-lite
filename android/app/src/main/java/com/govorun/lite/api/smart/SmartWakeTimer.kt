package com.govorun.lite.api.smart

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** One in-process wake for the same deadline as the durable Android job. No network: wait for its existing callback. */
internal class SmartWakeTimer(
    private val scope: CoroutineScope,
    private val clock: () -> Long = System::currentTimeMillis,
    private val online: () -> Boolean,
    private val wake: () -> Unit,
) {
    private var waiting: Job? = null

    @Synchronized
    fun schedule(atMs: Long?) {
        waiting?.cancel()
        waiting = atMs?.let {
            scope.launch {
                delay((it - clock()).coerceAtLeast(0))
                if (online()) wake()
            }
        }
    }
}
