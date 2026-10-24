package com.govorun.lite.ui.home

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationDrawerItem
import androidx.compose.material3.Text
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.GearSix
import com.adamglin.phosphoricons.regular.ClockClockwise
import com.adamglin.phosphoricons.regular.Info
import com.adamglin.phosphoricons.regular.SquaresFour
import com.adamglin.phosphoricons.regular.Star
import com.adamglin.phosphoricons.regular.UserCircle
import com.govorun.lite.BuildConfig
import com.govorun.lite.R
import com.govorun.lite.network.NetworkPolicy
import com.product.ui.SettingsSwitchRow
import com.product.ui.ProductTokens
import kotlinx.coroutines.launch

@Composable
internal fun HomeDrawerLayout(
    account: AccountSummaryState,
    network: NetworkPolicy.Choice,
    onEvent: (HomeEvent) -> Unit,
    content: @Composable (openDrawer: () -> Unit) -> Unit,
) {
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    val navigate: (HomeEvent) -> Unit = { event ->
        scope.launch {
            drawerState.close()
            onEvent(event)
        }
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet(
                modifier = Modifier.fillMaxHeight().widthIn(max = 320.dp),
            ) {
                DrawerHeader()
                HorizontalDivider(
                    modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Section),
                    color = MaterialTheme.colorScheme.outlineVariant,
                )
                Column(modifier = Modifier.padding(vertical = ProductTokens.Spacing.Lg)) {
                    DrawerItem(
                        PhosphorIcons.Regular.UserCircle,
                        if (account.signedIn) account.displayName else "Войти",
                    ) { navigate(HomeEvent.OpenAccount) }
                    DrawerItem(
                        PhosphorIcons.Regular.GearSix,
                        "Настройки",
                    ) { navigate(HomeEvent.OpenSettings) }
                    DrawerItem(
                        PhosphorIcons.Regular.Star,
                        "Оставить отзыв",
                    ) { navigate(HomeEvent.OpenReview) }
                    DrawerItem(
                        PhosphorIcons.Regular.SquaresFour,
                        "Наши продукты",
                    ) { navigate(HomeEvent.OpenProducts) }
                    DrawerItem(
                        PhosphorIcons.Regular.Info,
                        "О программе",
                    ) { navigate(HomeEvent.OpenAbout) }
                }
                HorizontalDivider(
                    modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Section),
                    color = MaterialTheme.colorScheme.outlineVariant,
                )
                SettingsSwitchRow(
                    title = "Офлайн",
                    summary = "Говорун не выходит в интернет",
                    checked = network.offline,
                    onCheckedChange = { onEvent(HomeEvent.SetOffline(it)) },
                )
                Spacer(Modifier.weight(1f))
                Text(
                    "Говорун · ${BuildConfig.VERSION_NAME}",
                    modifier = Modifier.padding(ProductTokens.Spacing.Section),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
    ) {
        content { scope.launch { drawerState.open() } }
    }
}

@Composable
private fun DrawerHeader() {
    Row(
        modifier = Modifier.padding(
            horizontal = ProductTokens.Spacing.Section,
            vertical = ProductTokens.Spacing.Screen,
        ),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            ImageVector.vectorResource(R.drawable.logo_bird),
            contentDescription = null,
            modifier = Modifier.size(40.dp),
            tint = MaterialTheme.colorScheme.primary,
        )
        Column(modifier = Modifier.padding(start = ProductTokens.Spacing.Xl)) {
            Text(
                "Говорун",
                style = MaterialTheme.typography.titleLarge,
            )
            Text(
                "Голосовой ввод без интернета",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun DrawerItem(
    icon: ImageVector,
    title: String,
    onClick: () -> Unit,
) {
    NavigationDrawerItem(
        icon = {
            Icon(icon, contentDescription = null, modifier = Modifier.size(20.dp))
        },
        label = { Text(title, style = MaterialTheme.typography.bodyMedium) },
        selected = false,
        onClick = onClick,
        modifier = Modifier
            .padding(horizontal = ProductTokens.Spacing.Lg)
            .heightIn(min = ProductTokens.Size.Row),
    )
}
