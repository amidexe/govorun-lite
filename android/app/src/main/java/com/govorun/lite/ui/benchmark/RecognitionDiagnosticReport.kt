package com.govorun.lite.ui.benchmark

/** Only measurements and fixed outcomes; exception messages and journal events are never exported. */
internal object RecognitionDiagnosticReport {
    val stages = listOf("fixtures", "model", "cold", "warm", "long")
    private val labels = listOf("Примеры и контрольные суммы", "Загрузка модели",
        "Первое распознавание", "Повторное распознавание", "Длинная запись")

    fun render(system: String, state: String, stage: String, result: String = ""): String {
        val current = stages.indexOf(stage).coerceAtLeast(0)
        val checks = labels.mapIndexed { index, label ->
            val status = when {
                state == "done" || (state != "idle" && index < current) -> "OK"
                index == current && state == "failed" -> "ОШИБКА"
                index == current && state == "running" -> "выполняется"
                index == current && state == "cancelled" -> "отменено"
                else -> "не проверено"
            }
            "$label: $status"
        }
        return buildString {
            appendLine("Говорун — диагностический отчёт v1")
            appendLine(system)
            appendLine("Набор: gigaam-samples-v1 · 11290 / 71250 мс")
            appendLine("Движок: sherpa-onnx · GigaAM v3 E2E RNNT · ONNX")
            appendLine("Доступно в приложении: CPU (4 потока). GPU/NNAPI не используются этой сборкой.")
            appendLine()
            checks.forEach(::appendLine)
            if (state == "failed") appendLine("Код: ${failureCode(stage)} · этап: $stage")
            if (result.isNotEmpty()) appendLine(result)
            appendLine("Микрофон, клавиатура, вставка и онлайн-обработка не тестировались.")
            appendLine("Это скорость Говоруна на одинаковом аудио; движки Android и desktop различаются.")
            appendLine("Успешное распознавание подтверждает эту нагрузку, а не исправность всех драйверов.")
        }
    }

    fun failureCode(stage: String) = when (stage) {
        "fixtures" -> "samples_unavailable"
        "model" -> "initialization_failed"
        else -> "recognition_failed"
    }
}
