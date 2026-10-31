package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CalendarBlank
import com.adamglin.phosphoricons.regular.DotOutline
import com.adamglin.phosphoricons.regular.ListChecks
import com.adamglin.phosphoricons.regular.NotePencil
import com.adamglin.phosphoricons.regular.ShieldCheck
import com.adamglin.phosphoricons.regular.Sparkle
import com.adamglin.phosphoricons.regular.Star
import com.adamglin.phosphoricons.regular.Waveform
import com.govorun.lite.account.ActiveStyleLimitPresentation
import com.govorun.lite.account.PromoProgram
import com.product.ui.ProductTokens
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
    val joined = program.participated && program.state == "active"
    BillingCard {
        // The name with its end beside it, then what it includes, then — for a participant — what is left.
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Промо", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            promoProgramDescription(program).removePrefix("Действует ").takeIf(String::isNotBlank)?.let {
                Text(it, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        // What Promo gives, as the server says it (program control); no lines from the server, no lines here.
        if (program.state == "active" && program.features.isNotEmpty()) {
            Column(Modifier.padding(top = ProductTokens.Spacing.Md), verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
                program.features.forEach { PromoFeature(featureIcon(it.icon), it.title, it.detail) }
            }
        }
        // The server's own words only when Promo is paused or over; while it runs, the lines above say it all.
        if (program.state != "active") program.message.takeIf(String::isNotBlank)?.let {
            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (!program.participated && program.state == "active" && !program.enrollmentOpen) {
            Text("Набор участников временно закрыт", style = MaterialTheme.typography.bodyMedium)
        }
        if (program.canActivate) {
            Button(onClick = onActivate, enabled = !working, modifier = Modifier.fillMaxWidth()) {
                Text(if (working) "Подключаем…" else "Подключить")
            }
        }
        if (joined && limit != null) {
            HorizontalDivider(Modifier.padding(vertical = ProductTokens.Spacing.Md), color = MaterialTheme.colorScheme.outlineVariant)
            ActiveStyleLimitIndicators(limit, compact = true)
        }
    }
}

private fun featureIcon(key: String): ImageVector = when (key) {
    "sparkle" -> PhosphorIcons.Regular.Sparkle
    "echo" -> PhosphorIcons.Regular.Waveform
    "tasks" -> PhosphorIcons.Regular.ListChecks
    "notes" -> PhosphorIcons.Regular.NotePencil
    "calendar" -> PhosphorIcons.Regular.CalendarBlank
    "shield" -> PhosphorIcons.Regular.ShieldCheck
    "star" -> PhosphorIcons.Regular.Star
    else -> PhosphorIcons.Regular.DotOutline
}

/** One thing the tariff includes: a quiet icon, what it is, and how much in a short line. */
@Composable
private fun PromoFeature(icon: ImageVector, title: String, detail: String) {
    Row(horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg), verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, contentDescription = null, modifier = Modifier.size(22.dp), tint = MaterialTheme.colorScheme.primary)
        Column {
            Text(title, style = MaterialTheme.typography.bodyMedium)
            if (detail.isNotEmpty()) Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
