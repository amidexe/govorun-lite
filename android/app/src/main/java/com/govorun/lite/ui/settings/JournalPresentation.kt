package com.govorun.lite.ui.settings

import android.content.Context
import com.govorun.lite.account.ACCOUNT_SESSION_ENDED_MESSAGE
import com.govorun.lite.diagnostics.LifecycleDiagnosticReason
import com.govorun.lite.stylization.diagnosticOrElse
import com.govorun.lite.stylization.LocalDiagnosticEntry
import com.govorun.lite.stylization.StylizationTimingFormat
import com.govorun.lite.stylization.LocalDiagnosticJournal
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

internal fun localDiagnosticRows(context: Context) = diagnosticOrElse(listOf(unavailableDiagnosticRow())) {
    journalRows(LocalDiagnosticJournal.read(context))
}

internal fun journalText(entries: List<LocalDiagnosticEntry>, zone: ZoneId = ZoneId.systemDefault()): String {
    val ordered = entries.sortedByDescending { it.timeMillis }
    val formatter = DateTimeFormatter.ofPattern("dd.MM.yyyy HH:mm:ss").withZone(zone)
    return journalRows(ordered, zone).zip(ordered).joinToString("\n") { (row, entry) ->
        diagnosticOrElse("Журнал · Не удалось прочитать запись журнала") {
            val message = listOfNotNull(row.title, row.detail).joinToString(". ")
            "${formatter.format(Instant.ofEpochMilli(entry.timeMillis))} · ${row.category} · $message" +
                (entry.timing?.let { "\n$it" } ?: "")
        }
    }
}

internal fun journalRows(
    entries: List<LocalDiagnosticEntry>,
    zone: ZoneId = ZoneId.systemDefault(),
    today: LocalDate = LocalDate.now(zone),
): List<DiagnosticEventRow> = entries.sortedByDescending { it.timeMillis }.map { entry ->
    diagnosticOrElse(unavailableDiagnosticRow()) { diagnosticRow(entry, zone, today) }
}

private fun unavailableDiagnosticRow() = DiagnosticEventRow(
    day = "Диагностика", time = "", title = "Не удалось прочитать запись журнала",
    detail = null, category = "Журнал", attention = true, success = false,
)

private fun diagnosticRow(entry: LocalDiagnosticEntry, zone: ZoneId, today: LocalDate): DiagnosticEventRow {
    val date = Instant.ofEpochMilli(entry.timeMillis).atZone(zone)
    val day = when (date.toLocalDate()) {
        today -> "Сегодня"
        today.minusDays(1) -> "Вчера"
        else -> date.format(DateTimeFormatter.ofPattern("d MMMM yyyy", Locale.forLanguageTag("ru")))
    }
    val time = date.format(DateTimeFormatter.ofPattern("HH:mm:ss"))
    entry.styleAccess?.let {
        val received = it == com.govorun.lite.stylization.StyleAccessDiagnostic.TOKENS_RECEIVED
        return DiagnosticEventRow(day = day, time = time, title = it.message,
            detail = null, category = "Стилизация", attention = !received, success = received)
    }
    com.govorun.lite.stylization.SelfHostedDiagnostic.parse(entry.timing)?.let {
        return DiagnosticEventRow(day = day, time = time, title = it.title,
            detail = "${it.elapsedMs} мс", category = "Свой сервер",
            attention = !it.success && it.result != "cancelled", success = it.success)
    }
    if (entry.timing != null || entry.state != null) {
        val view = StylizationTimingFormat.presentation(entry.timing, entry.state)
        return DiagnosticEventRow(day = day, time = time, title = view.title,
            detail = view.details, compact = view.compact, category = view.category,
            attention = view.attention, success = view.success,
            expandable = view.details != null)
    }
    return lifecycleRow(checkNotNull(entry.lifecycle), day, time)
}

private fun lifecycleRow(reason: LifecycleDiagnosticReason, day: String, time: String): DiagnosticEventRow {
    val checkEvent = reason.name.startsWith("CHECK_")
    val title = when (reason) {
        LifecycleDiagnosticReason.CHECK_COMPLETED,
        LifecycleDiagnosticReason.CHECK_SAMPLES_FAILED,
        LifecycleDiagnosticReason.CHECK_MODEL_FAILED,
        LifecycleDiagnosticReason.CHECK_RECOGNITION_FAILED -> lifecycleDiagnosticSummary(reason)
        LifecycleDiagnosticReason.SERVICE_CONNECTED -> "Птичка подключена"
        LifecycleDiagnosticReason.SERVICE_STOPPED -> "Служба птички остановлена"
        LifecycleDiagnosticReason.SERVICE_INTERRUPTED -> "Android запросил прерывание службы"
        LifecycleDiagnosticReason.EXIT_PACKAGE_UPDATED -> "Приложение обновлено"
        LifecycleDiagnosticReason.EXIT_CRASH,
        LifecycleDiagnosticReason.EXIT_NATIVE_CRASH -> "Сбой приложения"
        LifecycleDiagnosticReason.EXIT_HISTORY_UNAVAILABLE -> "История остановок недоступна"
        LifecycleDiagnosticReason.ACCOUNT_ENDED_BY_SERVER -> "Сервер завершил вход"
        LifecycleDiagnosticReason.STORAGE_MOVED,
        LifecycleDiagnosticReason.STORAGE_LEGACY_REIMPORTED,
        LifecycleDiagnosticReason.STORAGE_RESTORED_FROM_BACKUP,
        LifecycleDiagnosticReason.STORAGE_DATABASE_REPLACED -> "Перенос локальных данных"
        else -> "Приложение остановлено"
    }
    val detail = when (reason) {
        LifecycleDiagnosticReason.CHECK_COMPLETED,
        LifecycleDiagnosticReason.CHECK_SAMPLES_FAILED,
        LifecycleDiagnosticReason.CHECK_MODEL_FAILED,
        LifecycleDiagnosticReason.CHECK_RECOGNITION_FAILED -> null
        LifecycleDiagnosticReason.SERVICE_CONNECTED,
        LifecycleDiagnosticReason.SERVICE_STOPPED,
        LifecycleDiagnosticReason.SERVICE_INTERRUPTED -> null
        LifecycleDiagnosticReason.EXIT_PACKAGE_UPDATED -> "Android завершил предыдущую версию."
        LifecycleDiagnosticReason.EXIT_LOW_MEMORY -> "Android освободил память, завершив приложение."
        LifecycleDiagnosticReason.EXIT_CRASH -> "Работа завершилась из-за ошибки."
        LifecycleDiagnosticReason.EXIT_NATIVE_CRASH -> "Ошибка в одном из компонентов приложения."
        LifecycleDiagnosticReason.EXIT_ANR -> "Приложение перестало отвечать на запросы Android."
        LifecycleDiagnosticReason.EXIT_SIGNALED,
        LifecycleDiagnosticReason.EXIT_UNKNOWN -> "Android не сообщил точную причину."
        LifecycleDiagnosticReason.EXIT_HISTORY_UNAVAILABLE -> "Android не предоставил сведения о предыдущих остановках."
        LifecycleDiagnosticReason.ACCOUNT_ENDED_BY_SERVER -> ACCOUNT_SESSION_ENDED_MESSAGE
        LifecycleDiagnosticReason.STORAGE_MOVED,
        LifecycleDiagnosticReason.STORAGE_LEGACY_REIMPORTED,
        LifecycleDiagnosticReason.STORAGE_RESTORED_FROM_BACKUP,
        LifecycleDiagnosticReason.STORAGE_DATABASE_REPLACED -> "Код: ${reason.name.lowercase()}."
        else -> lifecycleDiagnosticSummary(reason)
    }
    val attention = (checkEvent && reason != LifecycleDiagnosticReason.CHECK_COMPLETED) || reason in setOf(
        LifecycleDiagnosticReason.EXIT_LOW_MEMORY, LifecycleDiagnosticReason.EXIT_CRASH,
        LifecycleDiagnosticReason.EXIT_NATIVE_CRASH, LifecycleDiagnosticReason.EXIT_ANR,
        LifecycleDiagnosticReason.EXIT_INITIALIZATION, LifecycleDiagnosticReason.EXIT_RESOURCE_USAGE,
    )
    val serviceEvent = reason in setOf(LifecycleDiagnosticReason.SERVICE_CONNECTED,
        LifecycleDiagnosticReason.SERVICE_STOPPED, LifecycleDiagnosticReason.SERVICE_INTERRUPTED)
    return DiagnosticEventRow(day, time, title, detail, attention,
        category = if (checkEvent) "Распознавание" else if (serviceEvent) "Птичка"
            else if (reason == LifecycleDiagnosticReason.ACCOUNT_ENDED_BY_SERVER) "Аккаунт" else "Приложение",
        success = reason == LifecycleDiagnosticReason.SERVICE_CONNECTED || reason == LifecycleDiagnosticReason.CHECK_COMPLETED)
}
