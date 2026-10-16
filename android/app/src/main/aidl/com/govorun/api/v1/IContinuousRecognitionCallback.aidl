package com.govorun.api.v1;

oneway interface IContinuousRecognitionCallback {
    void onReady(String sessionId);
    void onSpeechStateChanged(String sessionId, boolean active);
    void onFinalResult(String sessionId, long sequence, String text);
    void onStopped(String sessionId);
    void onError(String sessionId, int code, String safeMessage);
}
