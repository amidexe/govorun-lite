package com.govorun.lite.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.RadioButtonDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Sparkle
import com.govorun.lite.account.ActiveStyleLimitPresentation
import com.govorun.lite.account.BillingPlan
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard
import java.text.NumberFormat
import java.util.Locale

@Composable
internal fun PlanCard(
    plan: BillingPlan,
    active: Boolean,
    selected: Boolean,
    selectable: Boolean,
    trialDays: Int?,
    limit: ActiveStyleLimitPresentation?,
    onSelect: () -> Unit,
    actions: (@Composable () -> Unit)? = null,
    showPrice: Boolean = true,
) {
    val accent = MaterialTheme.colorScheme.primary
    PlanSelectionCard(
        selected = selected,
        selectable = selectable,
        onSelect = onSelect,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
                ) {
                    Text(
                        planName(plan),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    plan.clientMultiplierCopy?.let { multiplier ->
                        Text(
                            multiplier,
                            style = MaterialTheme.typography.titleMedium,
                            color = accent,
                        )
                    }
                }
                if (showPrice) PlanPrice(plan, trialDays)
                trialDays?.let { days ->
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
                    ) {
                        Icon(
                            PhosphorIcons.Regular.Sparkle,
                            contentDescription = null,
                            modifier = Modifier.size(ProductTokens.Size.SlotIcon),
                            tint = accent,
                        )
                        Text(
                            buildAnnotatedString {
                                withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) {
                                    append("$days дней")
                                }
                                append(" бесплатно")
                            },
                            style = MaterialTheme.typography.bodyMedium,
                            color = accent,
                        )
                    }
                }
                if (showPrice && trialDays != null) Text(
                    "Затем ${formatBillingKopeks(plan.priceKopeks)} за ${plan.billingPeriodLabel}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    planLimitCopy(plan),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            when {
                active -> Text(
                    "Текущий тариф",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.primary,
                )
                selectable -> RadioButton(
                    selected = selected,
                    onClick = null,
                    colors = RadioButtonDefaults.colors(selectedColor = accent),
                )
            }
        }
        if (active && limit != null) {
            ActiveStyleLimitIndicators(
                limit = limit,
                modifier = Modifier.padding(top = ProductTokens.Spacing.Lg),
            )
        }
        actions?.invoke()
    }
}

@Composable
private fun PlanPrice(plan: BillingPlan, trialDays: Int?) {
    val price = formatBillingKopeks(plan.priceKopeks)
    Row(
        modifier = if (trialDays != null) Modifier.clearAndSetSemantics {
            contentDescription = "Первые $trialDays дней — бесплатно. Обычная цена — $price за ${plan.billingPeriodLabel}."
        } else Modifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
    ) {
        if (trialDays != null) Text(
            price,
            textDecoration = TextDecoration.LineThrough,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            if (trialDays != null) "0 ₽" else price,
            style = MaterialTheme.typography.headlineSmall,
            color = MaterialTheme.colorScheme.primary,
        )
        if (trialDays == null) Text(
            "за ${plan.billingPeriodLabel}",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

internal fun planLimitCopy(
    plan: BillingPlan,
    locale: Locale = Locale.forLanguageTag("ru-RU"),
): String {
    return plan.clientLimitCopy
}

@Composable
internal fun PlanSelectionCard(
    selected: Boolean,
    selectable: Boolean,
    onSelect: () -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val gradient = if (colors.background.luminance() < 0.5f) {
        listOf(colors.surfaceContainerHigh, colors.surfaceContainerLow)
    } else {
        listOf(colors.surfaceContainerLowest, colors.surfaceContainer)
    }
    val shape = RoundedCornerShape(ProductTokens.Radius.Card)
    Column(
        modifier = Modifier
            .productFeedCard()
            .clip(shape)
            .background(Brush.verticalGradient(gradient))
            .then(
                if (selectable) {
                    Modifier.selectable(
                        selected = selected,
                        onClick = onSelect,
                        role = Role.RadioButton,
                    )
                } else {
                    Modifier
                },
            )
            .padding(ProductTokens.Spacing.Section),
        content = content,
    )
}
