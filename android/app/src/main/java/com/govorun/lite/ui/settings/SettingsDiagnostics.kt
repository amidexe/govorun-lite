package com.govorun.lite.ui.settings

import com.govorun.lite.stylization.StylizationDiagnosticReason

internal fun diagnosticReasonSummary(reason: StylizationDiagnosticReason): String = when (reason) {
    StylizationDiagnosticReason.SUCCESS -> "результат вставлен"
    StylizationDiagnosticReason.STREAMING_MODE -> "потоковый режим"
    StylizationDiagnosticReason.CUSTOM_STYLE_DRAFT -> "редактирование своего стиля"
    StylizationDiagnosticReason.STYLIZATION_DISABLED -> "стилизация выключена"
    StylizationDiagnosticReason.TERMS_UPDATE_REQUIRED -> "нужно принять условия"
    StylizationDiagnosticReason.SERVICE_UNAVAILABLE -> "сервис недоступен"
    StylizationDiagnosticReason.ACCESS_INACTIVE -> "нет активного доступа"
    StylizationDiagnosticReason.SHORT_PHRASE -> "короткая фраза"
    StylizationDiagnosticReason.TEXT_TOO_LONG -> "слишком длинный текст"
    StylizationDiagnosticReason.NOT_CONFIGURED -> "сервис не настроен"
    StylizationDiagnosticReason.APP_RULE_DISABLED -> "выключено для приложения"
    StylizationDiagnosticReason.OFFLINE,
    StylizationDiagnosticReason.CONFIRMED_OFFLINE,
    -> "нет сети"
    StylizationDiagnosticReason.NO_CHANGE -> "ответ не изменил текст"
    StylizationDiagnosticReason.STYLE_PASS_RESERVE_EMPTY -> "резерв пропусков пуст"
    StylizationDiagnosticReason.STYLE_PASS_ISSUE_FAILED -> "не удалось получить пропуски"
    StylizationDiagnosticReason.STYLE_PASS_RECOVERY_PENDING -> "пропуски восстанавливаются"
    StylizationDiagnosticReason.ACCOUNT_STATE_CHANGED -> "состояние аккаунта изменилось"
    StylizationDiagnosticReason.TRANSPORT_FAILED -> "сбой соединения"
    StylizationDiagnosticReason.REQUEST_FAILED -> "запрос отклонён"
    StylizationDiagnosticReason.SERVER_BUSY -> "сервер занят"
    StylizationDiagnosticReason.SERVER_TIMEOUT -> "превышено время ожидания"
    StylizationDiagnosticReason.SERVER_UNAVAILABLE -> "сервер недоступен"
    StylizationDiagnosticReason.INVALID_RESPONSE -> "некорректный ответ сервера"
    StylizationDiagnosticReason.INSERTION_FAILED -> "не удалось вставить текст"
}
