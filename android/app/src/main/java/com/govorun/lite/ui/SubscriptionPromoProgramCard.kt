package com.govorun.lite.ui

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.govorun.lite.account.ActiveStyleLimitPresentation
import com.govorun.lite.account.PromoProgram
import java.time.ZoneId
import java.util.Locale

internal fun promoProgramDescription(program: PromoProgram): String {
    if (program.state != "active") return if (program.message.isNotBlank()) "" else when (program.state) {
        "paused" -> "Приостановлено"
        "ended" -> "Завершено"
        "scheduled" -> "Скоро"
        else -> ""
    }
    val deadline = program.endsAt ?: program.accessEndsAt
    val date = deadline?.let { formatSubscriptionDate(it, ZoneId.systemDefault(), Locale.forLanguageTag("ru")) }
    return when {
        date != null -> "Действует до $date"
        program.accessMode == "from_activation" && program.accessDays > 0 ->
            "Срок — ${program.accessDays} дней после подключения"
        else -> ""
    }
}

@Composable
internal fun PromoProgramCard(
    program: PromoProgram,
    limit: ActiveStyleLimitPresentation?,
    working: Boolean,
    onActivate: () -> Unit,
) {
    BillingCard {
        Text("Промо", style = MaterialTheme.typography.titleLarge)
        promoProgramDescription(program).takeIf(String::isNotBlank)?.let {
            Text(it, style = MaterialTheme.typography.bodyMedium)
        }
        program.message.takeIf(String::isNotBlank)?.let {
            Text(it, style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (!program.participated && program.state == "active" && !program.enrollmentOpen) {
            Text("Набор участников временно закрыт", style = MaterialTheme.typography.bodyMedium)
        }
        if (program.canActivate) {
            Button(onClick = onActivate, enabled = !working, modifier = Modifier.fillMaxWidth()) {
                Text(if (working) "Подключаем…" else "Подключить")
            }
        }
        if (program.participated && program.state == "active" && limit != null) {
            ActiveStyleLimitIndicators(limit, compact = true)
        }
    }
}
