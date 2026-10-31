package com.govorun.api.smart.v1;

/** Exactly one call per runTask. */
interface ISmartCallback {
    /** The task's JSON result. */
    void onResult(String resultJson);

    /**
     * code: a getStatus state, or rejected_input, server_error, timeout, cancelled.
     * Not before retryAfterMs (0: no wait named).
     */
    void onError(String code, String message, long retryAfterMs);
}
