package com.govorun.lite.service

import java.util.concurrent.CancellationException

/** A stale/disconnected Android editor must not crash the recording coroutine.
 * Never retry here: the editor may have written the text before throwing. */
internal fun attemptTextInsertion(
    insert: () -> Boolean,
    reportFailure: (RuntimeException) -> Unit = {},
): Boolean = try {
    insert()
} catch (cancelled: CancellationException) {
    throw cancelled
} catch (failure: RuntimeException) {
    reportFailure(failure)
    false
}
