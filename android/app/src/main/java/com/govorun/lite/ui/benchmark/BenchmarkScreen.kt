package com.govorun.lite.ui.benchmark

import android.graphics.Bitmap
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Duotone
import com.adamglin.phosphoricons.duotone.Info
import com.govorun.lite.R
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.ProductTopAction
import com.product.ui.SettingsInfoRow
import com.product.ui.SettingsSection
import com.product.ui.productScreenPadding

data class BenchmarkUiState(
    val running: Boolean = false,
    val stage: String = "fixtures",
    val report: String = "",
    val speed: String? = null,
    val rating: String? = null,
    val provider: String = "",
    val coldTime: String = "",
    val warmTime: String = "",
    val longTime: String = "",
    val device: String = "",
    val system: String = "",
    val error: String? = null,
)

@Composable
fun BenchmarkScreen(
    state: BenchmarkUiState,
    onBack: () -> Unit,
    onRun: () -> Unit,
    onShare: suspend (Bitmap) -> Unit,
    onSave: suspend (Bitmap) -> Unit,
    onCopy: () -> Unit = {},
    onCancel: () -> Unit = {},
) {
    var helpVisible by rememberSaveable { mutableStateOf(false) }
    if (state.speed != null) {
        BenchmarkResultScreen(
            state = state,
            onBack = onBack,
            onRun = onRun,
            onShare = onShare,
            onSave = onSave,
            onCopy = onCopy,
        )
        return
    }
    ProductScaffold(
        title = stringResource(R.string.benchmark_title),
        onBack = onBack,
        actions = listOf(
            ProductTopAction(PhosphorIcons.Duotone.Info, "Как считается") {
                helpVisible = true
            },
        ),
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                SettingsSection("Тест на этом устройстве") {
                    Text(
                        "Встроенные записи. CPU · Процессор. Без микрофона.",
                        modifier = Modifier.padding(ProductTokens.Spacing.Section),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (state.running) {
                        LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                        SettingsInfoRow(
                            title = when (state.stage) {
                                "fixtures" -> "Проверяем примеры…"
                                "model" -> "Загружаем модель…"
                                "cold" -> "1 из 3 · Первый запуск…"
                                "warm" -> "2 из 3 · Повторный запуск…"
                                else -> "3 из 3 · Длинная запись…"
                            },
                        )
                    }
                    state.error?.let {
                        ProductDivider()
                        Text(
                            it,
                            modifier = Modifier.padding(ProductTokens.Spacing.Section),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }
            item {
                Button(
                    onClick = if (state.running) onCancel else onRun,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = ProductTokens.Spacing.Sm),
                ) {
                    Text(if (state.running) "Отменить тест" else stringResource(R.string.benchmark_run))
                }
            }
            item { BenchmarkReportBlock(state, onCopy) }
        }
    }
    if (helpVisible) {
        AlertDialog(
            onDismissRequest = { helpVisible = false },
            title = { Text(stringResource(R.string.benchmark_help_title)) },
            text = { Text(stringResource(R.string.benchmark_help_body)) },
            confirmButton = {
                TextButton(onClick = { helpVisible = false }) {
                    Text(stringResource(R.string.benchmark_help_close))
                }
            },
        )
    }
}
