package com.govorun.lite.ui.home

import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.TextButton
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Badge
import androidx.compose.ui.semantics.stateDescription
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
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CloudSlash
import com.adamglin.phosphoricons.regular.Gear
import com.adamglin.phosphoricons.regular.SquaresFour
import com.adamglin.phosphoricons.regular.Star
import com.adamglin.phosphoricons.regular.UserCircle
import com.govorun.lite.network.NetworkPolicy
import kotlinx.coroutines.launch

@Composable
internal fun HomeDrawerLayout(
    account: AccountSummaryState,
    network: NetworkPolicy.Choice,
    onEvent: (HomeEvent) -> Unit,
    openNetworkRequest: Int,
    productsNotice: String? = null,
    content: @Composable (openDrawer: () -> Unit) -> Unit,
) {
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    androidx.activity.compose.BackHandler(drawerState.isOpen) { scope.launch { drawerState.close() } }
    var confirmOffline by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(openNetworkRequest) { if (openNetworkRequest > 0) drawerState.open() }
    val toggleOffline = {
        if (network.offline) onEvent(HomeEvent.SetOffline(false)) else confirmOffline = true
    }
    val navigate: (HomeEvent) -> Unit = { event ->
        scope.launch {
            drawerState.close()
            onEvent(event)
        }
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            // Offline stays directly above Settings, as in Echo, outside the scrolling navigation.
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
                        DrawerDivider()
                        DrawerItem(PhosphorIcons.Regular.Star, "Оставить отзыв") { navigate(HomeEvent.OpenReview) }
                        DrawerItem(PhosphorIcons.Regular.SquaresFour, "Наши продукты", notice = productsNotice,
                            trailing = { if (productsNotice != null) Badge(containerColor = MaterialTheme.colorScheme.primary) },
                        ) { navigate(HomeEvent.OpenProducts) }
                    }
                    DrawerDivider()
                    DrawerItem(
                        PhosphorIcons.Regular.CloudSlash, "Офлайн", checked = network.offline,
                        trailing = { Switch(checked = network.offline, onCheckedChange = null) },
                    ) { toggleOffline() }
                    DrawerItem(PhosphorIcons.Regular.Gear, "Настройки") { navigate(HomeEvent.OpenSettings) }
                    Spacer(Modifier.height(16.dp))
                }
            }
        },
    ) {
        content { scope.launch { drawerState.open() } }
    }
    if (confirmOffline) AlertDialog(
        onDismissRequest = { confirmOffline = false },
        title = { Text("Включить «Офлайн»?") },
        text = { Text("Умные функции будут недоступны.") },
        confirmButton = { TextButton(onClick = { confirmOffline = false; onEvent(HomeEvent.SetOffline(true)) }) { Text("Включить") } },
        dismissButton = { TextButton(onClick = { confirmOffline = false }) { Text("Отмена") } },
    )
}

@Composable
private fun DrawerHeader() {
    Text(
        "Говорун", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Normal,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 24.dp, end = 20.dp, top = 16.dp, bottom = 16.dp).semantics { heading() },
    )
}

@Composable
private fun DrawerDivider() {
    HorizontalDivider(
        modifier = Modifier.padding(horizontal = 20.dp, vertical = 10.dp),
        color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.45f),
    )
}

/** One row of the menu, as Echo's: a quiet outline icon, an ordinary label and, for a switch, the switch at the end. */
@Composable
private fun DrawerItem(
    icon: ImageVector,
    title: String,
    trailing: (@Composable () -> Unit)? = null,
    checked: Boolean? = null,
    notice: String? = null,
    onClick: () -> Unit,
) {
    Row(
        Modifier
            .fillMaxWidth()
            .semantics { if (notice != null) stateDescription = notice }
            .padding(horizontal = 8.dp)
            .clip(RoundedCornerShape(50))
            .then(if (checked == null) Modifier.clickable(onClick = onClick) else Modifier.toggleable(checked, role = androidx.compose.ui.semantics.Role.Switch, onValueChange = { onClick() }))
            .heightIn(min = 48.dp)
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(20.dp))
        Text(title, style = MaterialTheme.typography.bodyMedium, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
        trailing?.invoke()
    }
}
