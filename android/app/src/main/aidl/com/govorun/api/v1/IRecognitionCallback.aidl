package com.govorun.api.v1;

oneway interface IRecognitionCallback {
    void onFinalResult(String sessionId, String text);
    void onError(String sessionId, int code, String safeMessage);
}
