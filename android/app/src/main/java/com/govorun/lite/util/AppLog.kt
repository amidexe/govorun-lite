package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.BuildConfig
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * File-based journal, capped at MAX_LINES, trimmed from the top on overflow.
 *
 * Disabled entirely in release builds — release users can't read this file
 * (no run-as on non-debuggable APKs, no in-app viewer), and the log
 * occasionally captures 40-char snippets of recognised text when the
 * user's dictionary fires. Keeping it off in release is a privacy win
 * for zero practical loss: only a rooted attacker could ever read it,
 * and even then only the last ~30-50 dictation events. In debug builds
 * we keep writing — we (the developer) actually use the file via
 * `run-as` to diagnose flow during development.
 *
 * If you ever need diagnostic data from a release user, the right move
 * is an opt-in toggle in Settings + a "share log" button — not always-on
 * file logging.
 */
object AppLog {

    private const val FILE_NAME = "app_log.txt"
    private const val MAX_LINES = 500

    private val timeFormat = SimpleDateFormat("MM-dd HH:mm:ss", Locale.US)

    fun log(ctx: Context, message: String) {
        if (!BuildConfig.DEBUG) return
        try {
            val file = File(ctx.filesDir, FILE_NAME)
            val line = "${timeFormat.format(Date())}  $message\n"
            file.appendText(line)
            trimIfNeeded(file)
        } catch (_: Exception) {
        }
    }

    fun read(ctx: Context): String {
        val file = File(ctx.filesDir, FILE_NAME)
        return if (file.exists()) file.readText() else ""
    }

    fun clear(ctx: Context) {
        File(ctx.filesDir, FILE_NAME).delete()
    }

    private fun trimIfNeeded(file: File) {
        if (!file.exists()) return
        val lines = file.readLines()
        if (lines.size > MAX_LINES) {
            file.writeText(lines.takeLast(MAX_LINES).joinToString("\n") + "\n")
        }
    }
}
