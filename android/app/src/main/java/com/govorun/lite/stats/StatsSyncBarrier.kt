package com.govorun.lite.stats

/** Serializes local counters with account sync snapshots, resets and acknowledgements. */
internal object StatsSyncBarrier {
    val lock = Any()
}
