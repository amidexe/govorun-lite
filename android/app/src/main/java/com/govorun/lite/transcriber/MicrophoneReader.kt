package com.govorun.lite.transcriber

import java.util.concurrent.atomic.AtomicReference

/** Capture-worker device handoff. Stop can interrupt an open/read; close follows the worker's join. */
internal class MicrophoneReader<T : Any>(
    private val open: () -> T?,
    private val accepts: (T) -> Boolean,
    private val fallback: () -> Unit,
    private val readDevice: (T, ByteArray, Int, Int) -> Int,
    private val stopDevice: (T) -> Unit,
    private val releaseDevice: (T) -> Unit,
) {
    private val device = AtomicReference<T?>(null)
    @Volatile private var stopped = false

    fun read(target: ByteArray, offset: Int, count: Int): Int {
        repeat(2) {
            if (stopped) return 0
            val current = device.get() ?: (open() ?: return 0).also { device.set(it) }
            if (stopped) { release(); return 0 }
            val read = readDevice(current, target, offset, count)
            if (stopped) return 0
            if (read > 0 && accepts(current)) return read
            // Drop the stale buffer; the route owner distinguishes a handoff from a failed input.
            release()
            fallback()
        }
        return 0
    }

    fun stop() {
        stopped = true
        device.get()?.let { runCatching { stopDevice(it) } }
    }
    fun close() { stop(); release() }
    private fun release() {
        device.getAndSet(null)?.let {
            try { runCatching { stopDevice(it) } } finally { releaseDevice(it) }
        }
    }
}
