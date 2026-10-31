package com.govorun.lite.api.smart

import org.json.JSONArray
import org.json.JSONObject

/**
 * An app's smart task that Govorun keeps until its answer is collected (protocol 2). The app hands it in once; Govorun tries it
 * again when what stopped it is gone and tells the app when it changed, so no app keeps a queue, retries or Govorun's codes.
 * The app's text stays only while the task waits; after the answer only a digest of it is kept, so the same task with the same
 * text is answered from here and never paid twice.
 */
internal data class SmartJob(
    val owner: String,
    val taskId: String,
    val kind: String,
    /** A digest of the kind, style and text: the same task with another text is another task. */
    val hash: String,
    /** The app's `{"text", "style"?}`; null once the task has its answer. */
    val input: String?,
    /** The person pressed the function: only such a task waits for the person's own step. */
    val asked: Boolean,
    /** What it waits for (a Govorun code: no_network, limit_today, not_signed_in…); null once answered. */
    val wait: String? = null,
    /** The answer for the app: `{"text"}`, `{"digest"}`, or `{"code"}` when it ended without one. */
    val answer: String? = null,
    /** Waiting: when it is tried again ([PERSON]: on the person's step only). Answered: when it is forgotten. */
    val atMs: Long = 0,
    val tries: Int = 0,
) {
    val waiting: Boolean get() = answer == null
    /** It ended without a text: Govorun's code is the answer. */
    val failed: Boolean get() = answer != null && JSONObject(answer).has("code")

    fun json(): JSONObject = JSONObject().put("owner", owner).put("taskId", taskId).put("kind", kind).put("hash", hash)
        .put("input", input ?: JSONObject.NULL).put("asked", asked).put("wait", wait ?: JSONObject.NULL)
        .put("answer", answer ?: JSONObject.NULL).put("atMs", atMs).put("tries", tries)

    /** What the app's list shows: the task, its state and Govorun's words for why it waits. */
    fun listed(): JSONObject = JSONObject().put("taskId", taskId).put("kind", kind)
        .put("state", if (answer == null) "waiting" else if (JSONObject(answer).has("code")) "failed" else "done")
        .put("wait", wait ?: JSONObject.NULL).put("message", wait?.let(::waitWords) ?: "")

    companion object {
        /** Waiting for the person's own step: no timer, only their step in Govorun (or Govorun's start) tries it again. */
        const val PERSON = Long.MAX_VALUE

        fun of(value: JSONObject): SmartJob = SmartJob(value.getString("owner"), value.getString("taskId"), value.getString("kind"),
            value.getString("hash"), value.optString("input").takeUnless { value.isNull("input") }, value.getBoolean("asked"),
            value.optString("wait").takeUnless { value.isNull("wait") }, value.optString("answer").takeUnless { value.isNull("answer") },
            value.getLong("atMs"), value.getInt("tries"))

        fun list(jobs: Collection<SmartJob>): String = JSONArray().apply { jobs.forEach { put(it.json()) } }.toString()

        /** One bad entry is left out, never the others. */
        fun parse(text: String): List<SmartJob> = runCatching { JSONArray(text) }.getOrNull()?.let { array ->
            (0 until array.length()).mapNotNull { runCatching { of(array.getJSONObject(it)) }.getOrNull() }
        }.orEmpty()

        /** Govorun's words for a wait, as the app shows them; the person's own steps name the step. */
        fun waitWords(code: String): String = SmartState.entries.firstOrNull { it.wire == code }?.message?.takeIf { it.isNotEmpty() }
            ?: when (code) {
                SmartImprove.STYLIZATION_TERMS -> "Примите условия улучшения текста в Говоруне"
                SmartImprove.LIMIT_PERIOD -> "Лимит периода исчерпан"
                SmartApi.BUSY -> "Ждёт своей очереди"
                SmartApi.TIMEOUT -> "Сервис не ответил вовремя. Говорун повторит сам"
                SmartApi.SERVER_ERROR -> "Сервис не обработал запрос. Говорун повторит сам"
                else -> "Говорун повторит сам"
            }
    }
}
