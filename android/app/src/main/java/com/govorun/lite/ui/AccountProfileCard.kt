package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ArrowsClockwise
import com.adamglin.phosphoricons.regular.CheckCircle
import com.adamglin.phosphoricons.regular.DotsThreeVertical
import com.adamglin.phosphoricons.regular.UserCircle
import com.govorun.lite.account.AccountDataSyncMode
import com.govorun.lite.account.AccountState
import com.govorun.lite.account.userMessage
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard

@Composable
internal fun AccountProfileCard(
    state: AccountState,
    onSignOut: () -> Unit,
    onDeleteAccount: () -> Unit,
    onRequestDataSync: () -> Unit,
) {
    var menuOpen by rememberSaveable(state.accountId, state.signedIn) { mutableStateOf(false) }
    var confirmDeletion by rememberSaveable(state.accountId) { mutableStateOf(false) }
    if (confirmDeletion) {
        AlertDialog(
            onDismissRequest = { if (!state.deletingAccount) confirmDeletion = false },
            title = { Text("Удалить аккаунт?") },
            text = {
                Text(
                    "Будут удалены профиль Говоруна, синхронизированные данные и онлайн-доступ, " +
                        "включая остаток оплаченного периода. Локальная диктовка и данные на этом устройстве останутся. " +
                        "Если включено автопродление, сначала отключите его. Удаление не оформляет возврат денег. " +
                        "Необходимые сведения об оплатах и email для чеков сохраняются; " +
                        "по оплатам можно обратиться в support@govorun.club.",
                )
            },
            confirmButton = {
                TextButton(enabled = !state.deletingAccount, onClick = {
                    confirmDeletion = false
                    onDeleteAccount()
                }) { Text("Удалить аккаунт", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(enabled = !state.deletingAccount, onClick = { confirmDeletion = false }) { Text("Отмена") }
            },
        )
    }
    val colors = MaterialTheme.colorScheme
    Card(
        modifier = Modifier.productFeedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(containerColor = colors.surfaceContainerLow),
    ) {
        Box(modifier = Modifier.fillMaxWidth()) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 20.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Surface(shape = CircleShape, color = colors.primaryContainer) {
                    Box(modifier = Modifier.size(52.dp), contentAlignment = Alignment.Center) {
                        Icon(
                            PhosphorIcons.Regular.UserCircle, contentDescription = null,
                            modifier = Modifier.size(28.dp), tint = colors.onPrimaryContainer,
                        )
                    }
                }
                Text(
                    state.displayName,
                    modifier = Modifier.fillMaxWidth().padding(top = 10.dp),
                    style = MaterialTheme.typography.titleLarge,
                    textAlign = TextAlign.Center,
                )
                Row(
                    modifier = Modifier.padding(top = 6.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    val statusIcon = when {
                        state.deletingAccount || state.secureStorageProblem != null || state.message != null -> null
                        state.syncing || state.dataSyncMode == AccountDataSyncMode.EVALUATING -> PhosphorIcons.Regular.ArrowsClockwise
                        state.dataSyncMode == AccountDataSyncMode.ENABLED -> PhosphorIcons.Regular.CheckCircle
                        else -> null
                    }
                    statusIcon?.let {
                        Icon(it, contentDescription = null, modifier = Modifier.size(ProductTokens.Size.SlotIcon), tint = colors.primary)
                    }
                    Text(
                        profileSyncStatus(state), style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurfaceVariant, textAlign = TextAlign.Center,
                    )
                }
                if (state.secureStorageProblem == null && state.dataSyncMode == AccountDataSyncMode.LOCAL_ONLY) {
                    TextButton(onClick = onRequestDataSync, enabled = !state.deletingAccount) {
                        Text("Включить синхронизацию")
                    }
                }
                TextButton(
                    onClick = onSignOut, enabled = !state.deletingAccount,
                    colors = ButtonDefaults.textButtonColors(contentColor = colors.onSurfaceVariant),
                    modifier = Modifier.padding(top = 6.dp).heightIn(min = ProductTokens.Size.TouchTarget),
                ) {
                    Text("Выйти")
                }
            }
            Box(modifier = Modifier.align(Alignment.TopEnd).padding(4.dp)) {
                IconButton(onClick = { menuOpen = true }) {
                    Icon(
                        PhosphorIcons.Regular.DotsThreeVertical,
                        contentDescription = "Действия аккаунта",
                        modifier = Modifier.size(ProductTokens.Size.TopBarActionIcon),
                        tint = colors.onSurfaceVariant,
                    )
                }
                DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                    DropdownMenuItem(
                        text = { Text("Удалить аккаунт", color = colors.error) }, enabled = !state.deletingAccount,
                        onClick = { menuOpen = false; confirmDeletion = true },
                    )
                }
            }
        }
    }
}

private fun profileSyncStatus(state: AccountState): String = when {
    state.deletingAccount -> "Удаляем аккаунт…"
    state.secureStorageProblem != null -> "Аккаунт недоступен"
    state.message != null -> "Проверьте сообщение ниже"
    state.dataSyncMode == AccountDataSyncMode.EVALUATING -> "Проверяем синхронизацию…"
    state.dataSyncMode == AccountDataSyncMode.PENDING -> "Выберите настройки синхронизации"
    state.dataSyncMode == AccountDataSyncMode.LOCAL_ONLY -> "Синхронизация выключена"
    state.syncing -> "Синхронизируем…"
    else -> "Данные синхронизированы"
}

@Composable
internal fun AccountStatusCard(state: AccountState, onRetry: () -> Unit) {
    BillingCard {
        Text(
            state.secureStorageProblem?.userMessage ?: state.message.orEmpty(),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        OutlinedButton(onClick = onRetry, modifier = Modifier.padding(top = ProductTokens.Spacing.Lg)) { Text("Повторить") }
    }
}

@Composable
internal fun DataSyncDecision(
    onMergeLocalData: () -> Unit,
    onKeepLocalData: () -> Unit,
) {
    Text(
        "Объединить данные с аккаунтом?",
        style = MaterialTheme.typography.titleSmall,
    )
    Text(
        "Статистика, словарь и свои стили — на всех ваших устройствах.",
        modifier = Modifier.padding(top = ProductTokens.Spacing.Sm),
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
    Button(
        onClick = onMergeLocalData,
        modifier = Modifier.fillMaxWidth().padding(top = ProductTokens.Spacing.Xl),
    ) { Text("Объединить") }
    TextButton(
        onClick = onKeepLocalData,
        modifier = Modifier.fillMaxWidth().padding(top = ProductTokens.Spacing.Lg),
    ) { Text("Оставить на устройстве") }
}
