package com.govorun.lite.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import com.govorun.lite.account.AccountDevice
import com.govorun.lite.account.AccountState
import com.product.ui.ProductDivider
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

private data class PendingDeviceAction(val device: AccountDevice, val replacement: Boolean)

@Composable
internal fun DevicesCard(
    state: AccountState,
    onReplaceDevice: (String) -> Unit,
    onRevokeDevice: (String) -> Unit,
) {
    var pendingAction by remember(state.accountId, state.deviceLimitReached) {
        mutableStateOf<PendingDeviceAction?>(null)
    }
    pendingAction?.takeIf { action -> state.devices.any { it.id == action.device.id } }?.let { action ->
        AlertDialog(
            onDismissRequest = { pendingAction = null },
            title = {
                Text(if (action.replacement) "Удалить ${action.device.displayName} и войти?" else "Удалить ${action.device.displayName} из аккаунта?")
            },
            text = { Text(deviceConfirmationBody(action.device) +
                if (action.replacement) " Вместо него подключится этот телефон." else "") },
            confirmButton = {
                TextButton(enabled = !state.syncing, onClick = {
                    pendingAction = null
                    if (action.replacement) onReplaceDevice(action.device.id)
                    else onRevokeDevice(action.device.id)
                }) {
                    Text(if (action.replacement) "Удалить и войти" else "Удалить устройство")
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingAction = null }) { Text("Отмена") }
            },
        )
    }
    Card(
        modifier = Modifier.productFeedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(modifier = Modifier.padding(ProductTokens.Spacing.Section)) {
            Text(if (state.deviceLimitReached) "Подключение устройства" else "Устройства", style = MaterialTheme.typography.titleMedium)
            Text(
                if (state.deviceLimitReached) "Подключено ${state.deviceLimit} устройств. Чтобы войти, удалите одно из списка."
                else "${state.devices.size} из ${state.deviceLimit}. Удалите устройства, которыми больше не пользуетесь.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            state.devices.forEachIndexed { index, device ->
                if (index == 0) Spacer(Modifier.size(ProductTokens.Spacing.Xl)) else ProductDivider()
                DeviceRow(
                    device = device,
                    replacement = state.deviceLimitReached,
                    enabled = !state.syncing,
                    onAction = { pendingAction = PendingDeviceAction(device, state.deviceLimitReached) },
                )
            }
        }
    }
}

@Composable
private fun DeviceRow(
    device: AccountDevice,
    replacement: Boolean,
    enabled: Boolean,
    onAction: () -> Unit,
) {
    val modifier = Modifier.fillMaxWidth().padding(vertical = ProductTokens.Spacing.Lg)
    if (LocalDensity.current.fontScale >= 1.3f) {
        Column(modifier = modifier) {
            DeviceRowLabel(device)
            DeviceRowAction(device, replacement, enabled, onAction, Modifier.align(Alignment.End))
        }
    } else {
        Row(modifier = modifier, verticalAlignment = Alignment.CenterVertically) {
            DeviceRowLabel(device, Modifier.weight(1f))
            DeviceRowAction(device, replacement, enabled, onAction)
        }
    }
}

@Composable
private fun DeviceRowLabel(device: AccountDevice, modifier: Modifier = Modifier) {
    Column(modifier = modifier) {
        Text(device.displayName, style = MaterialTheme.typography.bodyMedium)
        Text(
            accountDeviceSubtitle(device),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        accountDeviceAddedAt(device)?.let { addedAt ->
            Text(addedAt, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun DeviceRowAction(
    device: AccountDevice,
    replacement: Boolean,
    enabled: Boolean,
    onAction: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val action = if (replacement) "Удалить и войти" else "Удалить устройство"
    OutlinedButton(
        onClick = onAction,
        enabled = enabled,
        modifier = modifier.semantics { contentDescription = "$action ${device.displayName}" },
    ) { Text(if (replacement) "Удалить и войти" else "Удалить") }
}

internal fun accountDeviceAddedAt(
    device: AccountDevice,
    now: Instant = Instant.now(),
    zone: ZoneId = ZoneId.systemDefault(),
): String? = formatDeviceMoment(device.activatedAt, now, zone, relative = false)?.let { "Добавлено — $it" }

internal fun accountDeviceSubtitle(
    device: AccountDevice,
    now: Instant = Instant.now(),
    zone: ZoneId = ZoneId.systemDefault(),
): String {
    if (device.current) return "Это устройство"
    val activity = formatDeviceMoment(device.lastSeenAt, now, zone, relative = true)
    return listOfNotNull(platformLabel(device.platform), activity).joinToString(" · ")
}

internal fun deviceConfirmationBody(
    device: AccountDevice,
    now: Instant = Instant.now(),
    zone: ZoneId = ZoneId.systemDefault(),
): String {
    val facts = listOfNotNull(
        formatDeviceMoment(device.lastSeenAt, now, zone, relative = true)?.let { "Последняя активность — $it." },
        formatDeviceMoment(device.activatedAt, now, zone, relative = false)?.let { "Добавлено — $it." },
    ).joinToString(" ")
    return listOf(
        facts,
        if (device.current) "Вы выйдете из аккаунта на этом устройстве." else "",
        "Доступ к аккаунту и онлайн-функциям будет отозван. Локальные данные на устройстве останутся.",
    ).filter(String::isNotBlank).joinToString(" ")
}

private fun formatDeviceMoment(
    value: String,
    now: Instant,
    zone: ZoneId,
    relative: Boolean,
): String? = runCatching {
    val moment = Instant.parse(value).atZone(zone)
    val today = now.atZone(zone).toLocalDate()
    when {
        relative && moment.toLocalDate() == today -> "сегодня, ${moment.format(TIME_FORMAT)}"
        relative && moment.toLocalDate() == today.minusDays(1) -> "вчера, ${moment.format(TIME_FORMAT)}"
        moment.year == today.year -> moment.format(DATE_FORMAT)
        else -> moment.format(DATE_WITH_YEAR_FORMAT)
    }
}.getOrNull()

private fun platformLabel(platform: String): String = when (platform) {
    "android" -> "Android"
    "macos" -> "macOS"
    "windows" -> "Windows"
    else -> "Устройство"
}

private val RU_LOCALE = Locale.forLanguageTag("ru")
private val TIME_FORMAT = DateTimeFormatter.ofPattern("HH:mm", RU_LOCALE)
private val DATE_FORMAT = DateTimeFormatter.ofPattern("d MMM, HH:mm", RU_LOCALE)
private val DATE_WITH_YEAR_FORMAT = DateTimeFormatter.ofPattern("d MMM yyyy, HH:mm", RU_LOCALE)
