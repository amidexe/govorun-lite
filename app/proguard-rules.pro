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
