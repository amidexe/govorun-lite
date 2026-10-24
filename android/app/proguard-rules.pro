# sherpa-onnx native bridge classes — must not be obfuscated.
-keep class com.k2fsa.sherpa.onnx.** { *; }
-keepclassmembers class com.k2fsa.sherpa.onnx.** { *; }

# Strip every android.util.Log call from release builds. Nothing in the app
# consumes logcat output (no crash reporter, no telemetry), so these calls
# are dead weight and a potential information-disclosure vector.
-assumenosideeffects class android.util.Log {
    public static int v(...);
    public static int d(...);
    public static int i(...);
    public static int w(...);
    public static int e(...);
    public static int wtf(...);
}

# Strip the on-device file journal in release. AppLog.log is a development
# aid only — AppLog.read/clear are unused, so R8 drops them via dead-code
# elimination once log() calls are gone.
-assumenosideeffects class com.govorun.lite.util.AppLog {
    public void log(android.content.Context, java.lang.String);
}
# Keep the API-33 bridge as a separate class. If R8 inlined its framework
# InputMethod calls into the common service/factory, Android 12 could try to
# resolve a class that does not exist on that OS before the SDK guard runs.
-keep class com.govorun.lite.service.Api33InputMethodBridge { *; }

# Stable JVM symbols are the only JNI boundary to the shared business core.
-keep class com.govorun.lite.shared.SharedBusiness { *; }
-keep class com.govorun.lite.shared.SharedAccountNative { *; }
-keep interface com.govorun.lite.shared.SharedAccountCallback { *; }
-keepclassmembers class * implements com.govorun.lite.shared.SharedAccountCallback {
    public void effect(long, java.lang.String);
    public void cancelEffect(long);
    public void complete(long, java.lang.String);
}
-keep class com.govorun.lite.shared.SharedDocumentsNative { *; }
-keepclassmembers class com.govorun.lite.storage.GovorunPreferences$Legacy {
    public java.lang.String legacy(java.lang.String);
    public void notice(java.lang.String);
}
