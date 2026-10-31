package com.govorun.lite.ui.home

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CloudSlash
import com.adamglin.phosphoricons.regular.GearSix
import com.adamglin.phosphoricons.regular.Info
import com.adamglin.phosphoricons.regular.SquaresFour
import com.adamglin.phosphoricons.regular.Star
import com.adamglin.phosphoricons.regular.UserCircle
import com.govorun.lite.R
import com.govorun.lite.network.NetworkPolicy
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
            // The side menu as Echo's: the name in spaced capitals, then plain rows, a section in small capitals, settings
            // alone at the bottom.
            ModalDrawerSheet(
                drawerContainerColor = MaterialTheme.colorScheme.background,
                modifier = Modifier.width(300.dp).fillMaxHeight(),
            ) {
                Column(Modifier.fillMaxHeight()) {
                    Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
                        DrawerHeader()
                        DrawerItem(
                            PhosphorIcons.Regular.UserCircle,
                            if (account.signedIn) account.displayName else "Войти",
                        ) { navigate(HomeEvent.OpenAccount) }
                        DrawerItem(PhosphorIcons.Regular.Star, "Оставить отзыв") { navigate(HomeEvent.OpenReview) }
                        DrawerItem(PhosphorIcons.Regular.SquaresFour, "Наши продукты") { navigate(HomeEvent.OpenProducts) }
                        DrawerItem(PhosphorIcons.Regular.Info, "О программе") { navigate(HomeEvent.OpenAbout) }
                        DrawerSection("Сеть")
                        DrawerItem(
                            PhosphorIcons.Regular.CloudSlash, "Офлайн",
                            trailing = { Switch(checked = network.offline, onCheckedChange = { onEvent(HomeEvent.SetOffline(it)) }) },
                        ) { onEvent(HomeEvent.SetOffline(!network.offline)) }
                    }
                    DrawerItem(PhosphorIcons.Regular.GearSix, "Настройки") { navigate(HomeEvent.OpenSettings) }
                    Spacer(Modifier.height(16.dp))
                }
            }
        },
    ) {
        content { scope.launch { drawerState.open() } }
    }
}

/** The name in spaced capitals beside the bird, as Echo's menu starts. */
@Composable
private fun DrawerHeader() {
    Row(
        Modifier.padding(start = 20.dp, top = 20.dp, bottom = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(
            ImageVector.vectorResource(R.drawable.logo_bird),
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.size(24.dp),
        )
        Text("ГОВОРУН", style = MaterialTheme.typography.labelLarge, letterSpacing = 2.5.sp)
    }
}

/** A section's name in small spaced capitals, as Echo's «СТАТИСТИКА». */
@Composable
private fun DrawerSection(title: String) {
    Text(
        title.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        letterSpacing = 1.8.sp,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 20.dp, end = 16.dp, top = 24.dp, bottom = 8.dp),
    )
}

/** One row of the menu, as Echo's: a quiet outline icon, an ordinary label and, for a switch, the switch at the end. */
@Composable
private fun DrawerItem(
    icon: ImageVector,
    title: String,
    trailing: (@Composable () -> Unit)? = null,
    onClick: () -> Unit,
) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp)
            .clip(RoundedCornerShape(ProductTokens.Radius.Sm))
            .clickable(onClick = onClick)
            .height(if (trailing == null) 44.dp else 52.dp)
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(20.dp))
        Text(title, style = MaterialTheme.typography.bodyMedium, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
        trailing?.invoke()
    }
}
