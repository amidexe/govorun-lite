package com.govorun.lite.storage

import android.content.Context
import android.content.SharedPreferences
import com.govorun.lite.diagnostics.LifecycleDiagnosticReason
import com.govorun.lite.shared.SharedDocumentsNative
import com.govorun.lite.stylization.LocalDiagnosticJournal
import java.util.WeakHashMap
import java.util.concurrent.Executors
import org.json.JSONArray
import org.json.JSONObject

/** The documents of govorun.db behind the SharedPreferences each feature already uses.
 * Keys and value types stay as they were; the shared base decides which document owns
 * a key and moves the old file once per change. Old files are never written again.
 * An unavailable base reads defaults and never stops dictation.
 */
internal object GovorunPreferences {
    private const val FILE = "govorun.db"
    private val bases = WeakHashMap<Context, Base>()

    fun of(context: Context, file: String): SharedPreferences =
        (context.applicationContext as? PreferenceFiles)?.preferenceFile(file) ?: base(context).file(file)

    /** Removes [keys] from the old file as well: a deleted secret leaves no copy behind
     * for an older version to bring back. */
    fun forgetOld(context: Context, file: String, keys: List<String>) {
        if ((context.applicationContext as? PreferenceFiles)?.preferenceFile(file) != null) return
        context.applicationContext.getSharedPreferences(file, Context.MODE_PRIVATE).edit()
            .apply { keys.forEach(::remove) }.commit()
    }

    fun unavailable(context: Context): Boolean =
        (context.applicationContext as? PreferenceFiles)?.preferenceFile(FILE) == null && base(context).unavailable()

    @Synchronized private fun base(context: Context): Base {
        val application = context.applicationContext
        return bases.getOrPut(application) { Base(application) }
    }

    /** A context that supplies its own preference files: JVM adapter tests only. */
    interface PreferenceFiles {
        fun preferenceFile(name: String): SharedPreferences?
    }

    private class Base(private val context: Context) {
        @Volatile private var writerThread: Thread? = null
        private val writer = Executors.newSingleThreadExecutor { task ->
            Thread(task, "govorun-documents").apply { isDaemon = true; writerThread = this }
        }
        private val handle = runCatching {
            if (SharedDocumentsNative.loaded()) SharedDocumentsNative.open(context.getDatabasePath(FILE).path, Legacy(::legacy, ::notice)) else 0L
        }.getOrDefault(0L)
        private val files = HashMap<String, Document>()

        @Synchronized fun file(name: String): SharedPreferences = files.getOrPut(name) { Document(this, name) }

        fun unavailable() = handle == 0L || runCatching { SharedDocumentsNative.unavailable() }.getOrDefault(true)

        /** The file's values, or null while the base cannot answer: a failed read is never kept. */
        fun load(file: String): MutableMap<String, Any>? {
            if (handle == 0L) return null
            val reply = runCatching { JSONObject(SharedDocumentsNative.load(handle, file)) }.getOrNull()
            if (reply?.optBoolean("available") != true) return null
            val values = reply.optJSONObject("values") ?: return null
            return values.keys().asSequence().mapNotNull { key -> decode(values.optJSONObject(key))?.let { key to it } }
                .toMap(mutableMapOf())
        }

        /** Writes run in order on one thread; commit waits for its own turn. Callers enqueue
         * under the document lock, so the base sees changes in the order memory took them. */
        fun write(changes: JSONArray): java.util.concurrent.Future<Boolean> {
            val task = java.util.concurrent.Callable {
                handle != 0L && runCatching {
                    JSONObject(SharedDocumentsNative.write(handle, changes.toString())).optBoolean("written")
                }.getOrDefault(false)
            }
            if (Thread.currentThread() == writerThread) return java.util.concurrent.CompletableFuture.completedFuture(task.call())
            return writer.submit(task)
        }

        fun notice(code: String) {
            val reason = when (code) {
                "storage_moved" -> LifecycleDiagnosticReason.STORAGE_MOVED
                "storage_legacy_reimported" -> LifecycleDiagnosticReason.STORAGE_LEGACY_REIMPORTED
                "storage_restored_from_backup" -> LifecycleDiagnosticReason.STORAGE_RESTORED_FROM_BACKUP
                "storage_database_replaced" -> LifecycleDiagnosticReason.STORAGE_DATABASE_REPLACED
                else -> return
            }
            // Recorded later: the journal itself lives in the base being opened.
            writer.execute { LocalDiagnosticJournal.record(context, reason) }
        }

        fun legacy(file: String): String = encode(context.getSharedPreferences(file, Context.MODE_PRIVATE).all)
    }

    /** Answers the shared base; JNI calls these public members by name. */
    class Legacy internal constructor(private val read: (String) -> String, private val report: (String) -> Unit) {
        fun legacy(file: String): String = read(file)
        fun notice(code: String) = report(code)
    }

    private class Document(private val base: Base, private val file: String) : SharedPreferences {
        private var loaded: MutableMap<String, Any>? = null
        /** While the base cannot answer: defaults and this session's changes, never written. */
        private val unsaved = HashMap<String, Any>()

        @Synchronized private fun values(): MutableMap<String, Any> =
            loaded ?: base.load(file)?.also { loaded = it; unsaved.clear() } ?: unsaved

        @Synchronized override fun getAll(): MutableMap<String, *> = HashMap(values())
        @Synchronized override fun getString(key: String, defValue: String?) = values()[key] as? String ?: defValue
        @Suppress("UNCHECKED_CAST")
        @Synchronized override fun getStringSet(key: String, defValues: MutableSet<String>?) =
            (values()[key] as? Set<String>)?.toMutableSet() ?: defValues
        @Synchronized override fun getInt(key: String, defValue: Int) = values()[key] as? Int ?: defValue
        @Synchronized override fun getLong(key: String, defValue: Long) = values()[key] as? Long ?: defValue
        @Synchronized override fun getFloat(key: String, defValue: Float) = values()[key] as? Float ?: defValue
        @Synchronized override fun getBoolean(key: String, defValue: Boolean) = values()[key] as? Boolean ?: defValue
        @Synchronized override fun contains(key: String) = values().containsKey(key)
        override fun registerOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) =
            throw UnsupportedOperationException("document listeners are not supported")
        override fun unregisterOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit
        override fun edit(): SharedPreferences.Editor = Editor()

        /** Applies to memory at once, then hands the same changes to the base. A file the
         * base never read is not written: its defaults would replace the stored values. */
        @Synchronized private fun publish(puts: Map<String, Any?>, clear: Boolean): java.util.concurrent.Future<Boolean>? {
            val values = values()
            val changes = JSONArray()
            if (clear) values.keys.filter { it !in puts }.forEach { changes.put(change(it, null)) }
            if (clear) values.clear()
            puts.forEach { (key, value) ->
                if (value == null) values.remove(key) else values[key] = value
                changes.put(change(key, value))
            }
            return if (loaded == null) null else base.write(changes)
        }

        private fun change(key: String, value: Any?) =
            JSONObject().put("file", file).put("key", key).put("value", value?.let(::encodeValue) ?: JSONObject.NULL)

        private inner class Editor : SharedPreferences.Editor {
            private val puts = LinkedHashMap<String, Any?>()
            private var clear = false
            override fun putString(key: String, value: String?) = apply { puts[key] = value }
            override fun putStringSet(key: String, values: MutableSet<String>?) = apply { puts[key] = values?.toSet() }
            override fun putInt(key: String, value: Int) = apply { puts[key] = value }
            override fun putLong(key: String, value: Long) = apply { puts[key] = value }
            override fun putFloat(key: String, value: Float) = apply { puts[key] = value }
            override fun putBoolean(key: String, value: Boolean) = apply { puts[key] = value }
            override fun remove(key: String) = apply { puts[key] = null }
            override fun clear() = apply { clear = true }
            override fun commit(): Boolean = take()?.let { runCatching { it.get() }.getOrDefault(false) } ?: false
            override fun apply() { take() }
            private fun take() = publish(LinkedHashMap(puts), clear).also { puts.clear(); clear = false }
        }
    }

    private fun encode(values: Map<String, *>): String = JSONObject().also { json ->
        values.forEach { (key, value) -> value?.let(::encodeValue)?.let { json.put(key, it) } }
    }.toString()

    private fun encodeValue(value: Any): JSONObject? = when (value) {
        is Boolean -> JSONObject().put("b", value)
        is Int -> JSONObject().put("i", value)
        is Long -> JSONObject().put("l", value)
        is Float -> JSONObject().put("f", value.toString())
        is String -> JSONObject().put("s", value)
        is Set<*> -> JSONObject().put("set", JSONArray(value.filterIsInstance<String>().sorted()))
        else -> null
    }

    private fun decode(value: JSONObject?): Any? = when {
        value == null -> null
        value.has("b") -> value.optBoolean("b")
        value.has("i") -> value.optInt("i")
        value.has("l") -> value.optLong("l")
        value.has("f") -> value.optString("f").toFloatOrNull()
        value.has("s") -> value.optString("s")
        value.has("set") -> value.optJSONArray("set")?.let { array -> (0 until array.length()).map(array::optString).toSet() }
        else -> null
    }
}
