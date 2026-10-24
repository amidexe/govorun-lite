package com.govorun.lite.shared

/** JNI symbols of the shared documents base; values cross as JSON text only. */
internal object SharedDocumentsNative {
    private val library = SharedLibrary { System.loadLibrary("govorun_jvm_bridge") }

    fun loaded(): Boolean = runCatching { library.requireAvailable() }.isSuccess

    /** [callback] answers `legacy(file): String` and `notice(code)`; 0 when the base cannot exist. */
    external fun open(path: String, callback: Any): Long
    external fun load(handle: Long, file: String): String
    external fun write(handle: Long, changes: String): String
    external fun unavailable(): Boolean
}
