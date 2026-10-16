package com.govorun.api.v1;

import android.app.PendingIntent;
import android.os.ParcelFileDescriptor;
import com.govorun.api.v1.IContinuousRecognitionCallback;
import com.govorun.api.v1.IRecognitionCallback;

interface IRecognitionService {
    int getProtocolVersion();
    int getAuthorizationStatus(String clientPackage);
    PendingIntent createAuthorizationRequest(String clientPackage);
    void recognize(
        String clientPackage,
        String sessionId,
        in ParcelFileDescriptor pcmAudio,
        int sampleRateHz,
        int channelCount,
        int bitsPerSample,
        IRecognitionCallback callback
    );
    void cancel(String clientPackage, String sessionId);
    boolean supportsContinuousRecognition();
    void recognizeContinuous(
        String clientPackage,
        String sessionId,
        in ParcelFileDescriptor pcmStream,
        int sampleRateHz,
        int channelCount,
        int bitsPerSample,
        IContinuousRecognitionCallback callback
    );
    void recognizeContinuousWithProfile(
        String clientPackage,
        String sessionId,
        in ParcelFileDescriptor pcmStream,
        int sampleRateHz,
        int channelCount,
        int bitsPerSample,
        int profile,
        IContinuousRecognitionCallback callback
    );
    int getCapabilities();
}
