package com.govorun.lite.ui.home

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import com.govorun.lite.stylization.hasInlineStylizationConsent
import com.govorun.lite.stylization.InlineStylizationConsentCard
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CaretRight
import com.adamglin.phosphoricons.regular.CheckCircle
import com.adamglin.phosphoricons.regular.Clock
import com.adamglin.phosphoricons.regular.List
import com.adamglin.phosphoricons.regular.GearSix
import com.adamglin.phosphoricons.regular.Microphone
import com.adamglin.phosphoricons.regular.Warning
import com.product.ui.FeedSectionHeader as ProductFeedSectionHeader
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.ProductTopAction
import com.product.ui.productFeedCard
import com.product.ui.productScreenPadding
import com.govorun.lite.ui.SubscriptionPaymentAlert

@Composable
internal fun HomeScreen(
    state: HomeUiState,
    paymentAlert: SubscriptionPaymentAlert? = null,
    notificationInvitation: (@Composable () -> Unit)? = null,
    styleJustEnabled: Boolean = false,
    onlineServicePaused: Boolean = false,
    onEvent: (HomeEvent) -> Unit,
) {
    val consentVisible = hasInlineStylizationConsent()
    HomeDrawerLayout(state.account, onEvent) { openDrawer ->
        ProductScaffold(
            title = "Говорун",
            navigationAction = ProductTopAction(
                PhosphorIcons.Regular.List,
                "Меню",
                openDrawer,
            ),
            actions = listOf(ProductTopAction(PhosphorIcons.Regular.GearSix, "Настройки") {
                onEvent(HomeEvent.OpenSettings)
            }),
        ) { innerPadding ->
            LazyColumn(
                modifier = Modifier.fillMaxSize().padding(innerPadding),
                contentPadding = productScreenPadding(),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                item { ReadinessCard(state, onEvent) }
                paymentAlert?.let { alert ->
                    item { PaymentAlertCard(alert, onEvent) }
                }
                if (consentVisible) item { InlineStylizationConsentCard() }
                else when (state.stylization.homeCard) {
                    HomeStylizationCard.ACCESS_SETUP ->
                        item { SubscriptionCard(state.stylization, onEvent) }
                    HomeStylizationCard.STYLE_PICKER ->
                        item { StylePickerCard(state.stylization, onEvent, styleJustEnabled, onlineServicePaused) }
                    HomeStylizationCard.HIDDEN -> Unit
                }
                item { StatisticsCard(state.statistics, state.account.syncDecisionPending, onEvent) }
                if (shouldShowDictionaryCard(state.dictionary)) {
                    item { DictionaryCard(state.dictionary, onEvent) }
                }
                if (!consentVisible) notificationInvitation?.let { invitation -> item { invitation() } }
            }
        }
    }
}

@Composable
private fun PaymentAlertCard(
    alert: SubscriptionPaymentAlert,
    onEvent: (HomeEvent) -> Unit,
) {
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column(
            modifier = Modifier.padding(ProductTokens.Spacing.Section),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    PhosphorIcons.Regular.Warning,
                    contentDescription = null,
                    modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                    tint = MaterialTheme.colorScheme.error,
                )
                Text(alert.title, style = MaterialTheme.typography.titleSmall)
            }
            Text(
                alert.body,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            TextButton(onClick = { onEvent(HomeEvent.OpenSubscription) }) {
                Text("Открыть подписку")
            }
        }
    }
}

@Composable
private fun ReadinessCard(
    state: HomeUiState,
    onEvent: (HomeEvent) -> Unit,
) {
    when {
        state.issues.isNotEmpty() -> SetupCard(state.issues, onEvent)
        state.ready -> ReadyStatusCard()
        else -> CheckingStatusCard()
    }
}

@Composable
private fun ReadyStatusCard() {
    val successColor = if (MaterialTheme.colorScheme.background.luminance() > 0.5f) {
        ProductTokens.Color.Success
    } else {
        ProductTokens.Color.SuccessDark
    }
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = ProductTokens.Size.Row)
                .padding(
                    horizontal = ProductTokens.Spacing.Section,
                    vertical = ProductTokens.Spacing.Xl,
                ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                PhosphorIcons.Regular.CheckCircle,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                tint = successColor,
            )
            Spacer(Modifier.width(ProductTokens.Spacing.Xl))
            Text(
                "Всё готово",
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.weight(1f),
            )
            Text(
                "Говорун работает",
                style = MaterialTheme.typography.labelSmall,
                color = successColor,
            )
        }
    }
}

@Composable
private fun CheckingStatusCard() {
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = ProductTokens.Size.Row)
                .padding(
                    horizontal = ProductTokens.Spacing.Section,
                    vertical = ProductTokens.Spacing.Xl,
                ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                PhosphorIcons.Regular.Clock,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.width(ProductTokens.Spacing.Xl))
            Text(
                "Проверяем готовность…",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun SetupCard(
    issues: List<SetupIssue>,
    onEvent: (HomeEvent) -> Unit,
) {
    var expanded by rememberSaveable(issues.map(SetupIssue::name)) {
        mutableStateOf(true)
    }
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column {
            FeedSectionHeader(
                title = "Нужно настроить",
                icon = PhosphorIcons.Regular.Warning,
                count = issues.size,
                expanded = expanded,
                onToggle = { expanded = !expanded },
            )
            AnimatedVisibility(expanded) {
                Column {
                    CompactDivider()
                    issues.forEachIndexed { index, issue ->
                        if (index > 0) CompactDivider()
                        val content = issueContent(issue)
                        ActionRow(
                            icon = content.icon,
                            title = content.title,
                            body = content.body,
                            onClick = { onEvent(content.event) },
                        )
                    }
                }
            }
        }
    }
}

private data class IssueContent(
    val icon: ImageVector,
    val title: String,
    val body: String,
    val event: HomeEvent,
)

private fun issueContent(issue: SetupIssue): IssueContent = when (issue) {
    SetupIssue.MICROPHONE -> IssueContent(
        PhosphorIcons.Regular.Microphone,
        "Разрешите микрофон",
        "Аудио остаётся на телефоне",
        HomeEvent.OpenAppDetails,
    )
    SetupIssue.ACCESSIBILITY -> IssueContent(
        PhosphorIcons.Regular.CheckCircle,
        "Включите Говоруна",
        "Чтобы вставлять текст в активное поле",
        HomeEvent.OpenAccessibility,
    )
    SetupIssue.ACCESSIBILITY_RESTART -> IssueContent(
        PhosphorIcons.Regular.Warning,
        "Перезапустите Говоруна",
        "Выключите и снова включите службу",
        HomeEvent.OpenAccessibility,
    )
    SetupIssue.ACCESSIBILITY_SHORTCUT -> IssueContent(
        PhosphorIcons.Regular.Warning,
        "Отключите системную кнопку",
        "Она может случайно выключить Говоруна",
        HomeEvent.OpenAccessibility,
    )
    SetupIssue.BATTERY -> IssueContent(
        PhosphorIcons.Regular.Clock,
        "Разрешите работу в фоне",
        "Для стабильной диктовки",
        HomeEvent.RequestBattery,
    )
}

@Composable
internal fun ActionRow(
    icon: ImageVector,
    title: String,
    body: String,
    onClick: () -> Unit,
) {
    Surface(onClick = onClick, color = Color.Transparent) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = ProductTokens.Size.Row)
                .padding(
                    horizontal = ProductTokens.Spacing.Section,
                    vertical = ProductTokens.Spacing.Md,
                ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                icon,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.width(ProductTokens.Spacing.Xl))
            Column(modifier = Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.bodyMedium)
                Text(
                    body,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Icon(
                PhosphorIcons.Regular.CaretRight,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

internal fun Modifier.feedCard(): Modifier = this.productFeedCard()

@Composable
internal fun FeedSectionHeader(
    title: String,
    icon: ImageVector? = null,
    trailingText: String? = null,
    count: Int? = null,
    expanded: Boolean? = null,
    onToggle: (() -> Unit)? = null,
    trailingTextColor: Color = MaterialTheme.colorScheme.onSurfaceVariant,
    actions: @Composable RowScope.() -> Unit = {},
) {
    ProductFeedSectionHeader(
        title = title,
        icon = icon,
        trailingText = trailingText,
        count = count,
        expanded = expanded,
        onToggle = onToggle,
        trailingTextColor = trailingTextColor,
        actions = actions,
    )
}

@Composable
internal fun CompactDivider() {
    ProductDivider()
}
