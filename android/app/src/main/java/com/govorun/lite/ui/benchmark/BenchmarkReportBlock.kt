package com.govorun.lite.ui.benchmark

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.product.ui.ProductTokens

@Composable
internal fun BenchmarkReportBlock(state: BenchmarkUiState, onCopy: () -> Unit) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(ProductTokens.Spacing.Sm)) {
        TextButton(onClick = onCopy, enabled = !state.running && state.report.isNotEmpty()) {
            Text("Скопировать отчёт")
        }
        TextButton(onClick = { expanded = !expanded }) {
            Text(if (expanded) "Скрыть технический отчёт" else "Технический отчёт")
        }
        if (expanded) Text(state.report, style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
