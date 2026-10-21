package com.govorun.api.v1;

oneway interface IRecognitionCallback {
    void onFinalResult(String sessionId, String text);
    void onError(String sessionId, int code, String safeMessage);

    /** Result of recognizeTimed: text and word times in milliseconds from the start of the audio. */
    void onFinalResultTimed(
        String sessionId,
        String text,
        in long[] wordStartsMs,
        in long[] wordEndsMs,
        in String[] words
    );
}
