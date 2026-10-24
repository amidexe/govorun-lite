package com.govorun.lite.ui.settings

import com.govorun.lite.diagnostics.LifecycleDiagnosticReason

internal fun birdConnectionNeedsAttention(enabled: Boolean, connected: Boolean): Boolean = enabled && !connected

internal fun lifecycleDiagnosticSummary(reason: LifecycleDiagnosticReason): String = when (reason) {
    LifecycleDiagnosticReason.CHECK_COMPLETED -> "Проверка распознавания завершена"
    LifecycleDiagnosticReason.CHECK_SAMPLES_FAILED -> "Проверка распознавания: примеры недоступны"
    LifecycleDiagnosticReason.CHECK_MODEL_FAILED -> "Проверка распознавания: ошибка загрузки модели"
    LifecycleDiagnosticReason.CHECK_RECOGNITION_FAILED -> "Проверка распознавания: ошибка обработки"
    LifecycleDiagnosticReason.SERVICE_CONNECTED -> "Служба птички подключена"
    LifecycleDiagnosticReason.SERVICE_STOPPED -> "Служба птички остановлена"
    LifecycleDiagnosticReason.SERVICE_INTERRUPTED -> "Android запросил прерывание службы птички"
    LifecycleDiagnosticReason.EXIT_LOW_MEMORY -> "Android завершил приложение: нехватка памяти"
    LifecycleDiagnosticReason.EXIT_CRASH -> "Приложение завершилось из-за сбоя"
    LifecycleDiagnosticReason.EXIT_NATIVE_CRASH -> "Приложение завершилось из-за сбоя нативного компонента"
    LifecycleDiagnosticReason.EXIT_ANR -> "Приложение завершено: не отвечало Android"
    LifecycleDiagnosticReason.EXIT_SELF -> "Приложение завершило свой процесс"
    LifecycleDiagnosticReason.EXIT_SIGNALED -> "Процесс завершён системным сигналом; точная причина неизвестна"
    LifecycleDiagnosticReason.EXIT_INITIALIZATION -> "Приложение завершено из-за ошибки запуска"
    LifecycleDiagnosticReason.EXIT_PERMISSION_CHANGED -> "Android завершил приложение после изменения разрешений"
    LifecycleDiagnosticReason.EXIT_RESOURCE_USAGE -> "Android завершил приложение: превышено использование ресурсов"
    LifecycleDiagnosticReason.EXIT_USER_REQUESTED -> "Остановка по запросу пользователя или изменению приложения"
    LifecycleDiagnosticReason.EXIT_USER_STOPPED -> "Android остановил профиль пользователя"
    LifecycleDiagnosticReason.EXIT_DEPENDENCY -> "Приложение завершено после остановки зависимого процесса"
    LifecycleDiagnosticReason.EXIT_OTHER -> "Android завершил приложение по другой системной причине"
    LifecycleDiagnosticReason.EXIT_FREEZER -> "Android завершил замороженный фоновый процесс"
    LifecycleDiagnosticReason.EXIT_PACKAGE_CHANGED -> "Приложение остановлено после отключения или изменения компонентов"
    LifecycleDiagnosticReason.EXIT_PACKAGE_UPDATED -> "Приложение остановлено для обновления"
    LifecycleDiagnosticReason.EXIT_UNKNOWN -> "Приложение завершилось; Android не сообщил причину"
    LifecycleDiagnosticReason.EXIT_HISTORY_UNAVAILABLE -> "Не удалось получить историю остановок от Android"
    LifecycleDiagnosticReason.ACCOUNT_ENDED_BY_SERVER -> "Сервер завершил вход на этом устройстве"
    LifecycleDiagnosticReason.STORAGE_MOVED,
    LifecycleDiagnosticReason.STORAGE_LEGACY_REIMPORTED,
    LifecycleDiagnosticReason.STORAGE_RESTORED_FROM_BACKUP,
    LifecycleDiagnosticReason.STORAGE_DATABASE_REPLACED -> "Перенос локальных данных"
}
