package com.govorun.lite.stylization

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withLink
import com.product.ui.FeedCard
import com.product.ui.ProductTokens

@Composable
internal fun StyleActivationCard(
    changesInputMode: Boolean, afterSignIn: Boolean,
    busy: Boolean, error: Boolean,
    onDocuments: () -> Unit, onCancel: () -> Unit, onAccept: () -> Unit,
    needsSubscription: Boolean = false, onPlans: () -> Unit = {},
) {
    BackHandler { if (!busy) onCancel() }
    FeedCard {
        Column(Modifier.padding(ProductTokens.Spacing.Section),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md)) {
            Text("Включить улучшение текста", style = MaterialTheme.typography.titleMedium)
            StyleBetaNotice()
            val linkStyle = TextLinkStyles(SpanStyle(color = MaterialTheme.colorScheme.onSurfaceVariant,
                textDecoration = TextDecoration.Underline))
            Text(buildAnnotatedString {
                append("Включая функцию, вы принимаете ")
                withLink(LinkAnnotation.Clickable("terms", linkStyle) { if (!busy) onDocuments() }) {
                    append("Условия и политика")
                }
                append(".")
            }, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (error) Text(if (needsSubscription) "Лимит улучшения текста закончился. Продолжить можно по подписке."
                else "Не удалось включить улучшение. Попробуйте ещё раз.",
                style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
            FlowRow(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                TextButton(enabled = !busy, onClick = onCancel) { Text("Не сейчас") }
                Button(enabled = !busy, onClick = { if (needsSubscription) onPlans() else onAccept() }) {
                    Text(if (busy) "Подключаем…" else if (needsSubscription) "Посмотреть тарифы" else "Принять и включить")
                }
            }
        }
    }
}
