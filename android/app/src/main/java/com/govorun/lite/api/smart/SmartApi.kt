package com.govorun.lite.api.smart

import com.govorun.lite.account.NO_INTERNET_MESSAGE

/** Говорун.Эхо's bridge to Govorun (AIDL com.govorun.api.smart.v1): status, Govorun's screens and «Улучшить через Говорун». */
internal object SmartApi {
    const val SERVICE_ACTION = "com.govorun.api.smart.v1.BIND"
    /** 2: tasks Govorun keeps ([QUEUED], [ACTION_TASKS_CHANGED], getTasks); Echo and Govorun are updated together. */
    const val PROTOCOL_VERSION = 2

    /** «Улучшить через Говорун»: the note's text through Govorun's own text improvement ([SmartImprove]), not an Echo task. */
    const val IMPROVE = "improve"
    /**
     * Итоги of a long note, one at a time: `{"text"}` → `{"digest": {nazvanie, o_chem, glavnoe, dela, resheniya, voprosy}}`;
     * dela, resheniya and voprosy are empty when the recording had none (a lecture).
     */
    const val DIGEST = "digest"
    /** What итоги of `{"text"}` cost, answered at once: `{characters, credits, allowed, reason, confirm, minChars, maxChars, limitToday, remainingToday}`. */
    const val DIGEST_PRICE = "digest_price"
    /** Govorun's «Улучшение текста» screen: the switch, the terms of online improvement and «Стили для приложений». */
    const val ACTION_STYLIZATION = "stylization"

    const val ACTION_SIGN_IN = "sign_in"
    const val ACTION_SUBSCRIPTION = "subscription"
    const val ACTION_NETWORK = "network"
    const val ACTION_AUTHORIZE = "authorize"
    /** Asks the person to give the app the lock-screen volume key ([RecognitionApi.SCOPE_VOLUME_KEY]). */
    const val ACTION_VOLUME_KEY = "volume_key"

    const val REJECTED_INPUT = "rejected_input"
    const val SERVER_ERROR = "server_error"
    const val TIMEOUT = "timeout"
    const val CANCELLED = "cancelled"
    const val BUSY = "busy"
    /** Protocol 2: Govorun keeps the task and tries it again by itself; the message says why it waits. */
    const val QUEUED = "queued"
    /** The model answered, but not as итоги: it was paid, so Govorun tries once more only. */
    const val MODEL_ANSWER = "model_answer"
    /** Protocol 2: a task the person did not press, while the person has not yet used the app's smart functions. */
    const val SMART_OFF = "smart_off"

    /** Protocol 2: sent to the app's package when one of its tasks has its answer or waits for something else. */
    const val ACTION_TASKS_CHANGED = "com.govorun.api.smart.v1.TASKS_CHANGED"
    const val EXTRA_TASK_ID = "taskId"

    const val MAX_TASK_ID = 64
    /** Encoded JSON budget, separate from the core's text-character limits; within Binder's parcel limit. */
    const val MAX_INPUT_BYTES = 196_608
    /** Improvements one app may have waiting in Govorun. */
    const val MAX_TASKS_PER_APP = 4

    /** In the status for Echo: Govorun improves notes (2.0.39+). */
    const val CAPABILITY_IMPROVE = "improve"
    /** In the status for Echo: a note runs in its own window, outlives Echo and is paid once. */
    const val CAPABILITY_NOTE_CLASS = "noteClass"
    /** In the status for Echo: Govorun makes итоги of long notes ([DIGEST], [DIGEST_PRICE]). */
    const val CAPABILITY_DIGEST = "digest"
    /** In the status for Echo: Govorun keeps tasks and tries them again by itself (protocol 2). */
    const val CAPABILITY_JOBS = "jobs"
}

/** Govorun's answer to «may the app ask now?», with Govorun's own words for it. */
internal enum class SmartState(val wire: String, val message: String) {
    READY("ready", ""),
    NOT_AUTHORIZED("not_authorized", "Разрешите умные функции в Говоруне"),
    NOT_SIGNED_IN("not_signed_in", "Умные функции работают через аккаунт Говоруна"),
    OFFLINE("offline", "В Говоруне сеть выключена в меню"),
    ROAMING("roaming", "Роуминг: включён офлайн"),
    NO_NETWORK("no_network", NO_INTERNET_MESSAGE),
    UNAVAILABLE("unavailable", "Временно недоступно"),
    NEEDS_ACCESS("needs_access", "Нужен доступ: Промо или подписка"),
    LIMIT_TODAY("limit_today", "Лимит на сегодня исчерпан, завтра снова");

    companion object {
        fun of(wire: String?): SmartState? = entries.firstOrNull { it.wire == wire }
    }
}

/** A refusal for the client's onError: a state or one of the task codes, never a server body. */
internal data class SmartRefusal(val code: String, val message: String, val retryAfterMs: Long = 0) {
    companion object {
        fun of(state: SmartState, retryAfterMs: Long = 0) = SmartRefusal(state.wire, state.message, retryAfterMs)
        fun of(code: String, retryAfterMs: Long = 0): SmartRefusal =
            SmartState.of(code)?.let { of(it, retryAfterMs) } ?: SmartRefusal(code, when (code) {
                SmartApi.REJECTED_INPUT -> "Запрос отклонён: неверный формат или параметры"
                SmartImprove.TOO_LONG -> "Превышен лимит размера запроса. Сократите текст или разделите заметку"
                SmartImprove.TOO_SHORT -> "Недостаточно текста для обработки"
                SmartImprove.LIMIT_PERIOD -> "Достигнут лимит за период. Подробнее — в Говоруне"
                SmartImprove.STYLIZATION_TERMS -> "Примите условия улучшения текста в Говоруне"
                SmartApi.MODEL_ANSWER -> "Сервис вернул результат, который не удалось использовать"
                SmartApi.TIMEOUT -> "Сервер не ответил вовремя"
                SmartApi.CANCELLED -> "Задача отменена"
                SmartApi.BUSY -> "Запрос ждёт своей очереди"
                SmartApi.SMART_OFF -> "Умные функции включаются, когда вы сами нажмёте их"
                else -> "Сервис временно не ответил"
            }, retryAfterMs)
    }
}
