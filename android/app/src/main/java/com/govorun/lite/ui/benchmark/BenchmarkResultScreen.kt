package com.govorun.lite.ui.benchmark

import android.graphics.Bitmap
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ArrowClockwise
import com.adamglin.phosphoricons.regular.DownloadSimple
import com.adamglin.phosphoricons.regular.ShareNetwork
import com.govorun.lite.R
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

@Composable
internal fun BenchmarkResultScreen(
    state: BenchmarkUiState,
    onBack: () -> Unit,
    onRun: () -> Unit,
    onShare: suspend (Bitmap) -> Unit,
    onSave: suspend (Bitmap) -> Unit,
    onCopy: () -> Unit = {},
) {
    val cardLayer = rememberGraphicsLayer()
    val scope = rememberCoroutineScope()
    var exporting by remember { mutableStateOf(false) }
    var exportError by remember { mutableStateOf<String?>(null) }
    val shareError = stringResource(R.string.benchmark_share_error)
    val saveError = stringResource(R.string.benchmark_save_error)
    fun export(saving: Boolean) {
        if (exporting) return
        exporting = true
        exportError = null
        scope.launch {
            var bitmap: Bitmap? = null
            try {
                val capture = cardLayer.toImageBitmap().asAndroidBitmap()
                bitmap = capture
                if (saving) onSave(capture) else onShare(capture)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                exportError = if (saving) saveError else shareError
            } finally {
                bitmap?.recycle()
                exporting = false
            }
        }
    }
    ProductScaffold(title = stringResource(R.string.benchmark_results_title), onBack = onBack) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(ProductTokens.Spacing.Section),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
        ) {
            BenchmarkResultCard(
                state = state,
                modifier = Modifier.drawWithContent {
                    cardLayer.record { this@drawWithContent.drawContent() }
                    drawLayer(cardLayer)
                },
            )
            ResultActions(
                enabled = !exporting,
                onShare = { export(saving = false) },
                onSave = { export(saving = true) },
                onRun = onRun,
            )
            if (exporting) Text(
                stringResource(R.string.benchmark_preparing_image),
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
            )
            exportError?.let {
                Text(it, color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
            }
            BenchmarkReportBlock(state, onCopy)
        }
    }
}

@Composable
private fun ResultActions(
    enabled: Boolean,
    onShare: () -> Unit,
    onSave: () -> Unit,
    onRun: () -> Unit,
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Row(horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
            Button(
                onClick = onShare,
                enabled = enabled,
                modifier = Modifier.weight(1f).heightIn(min = ProductTokens.Size.TouchTarget),
            ) {
                Icon(PhosphorIcons.Regular.ShareNetwork, contentDescription = null, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(ProductTokens.Spacing.Lg))
                Text(stringResource(R.string.benchmark_share))
            }
            OutlinedButton(
                onClick = onSave,
                enabled = enabled,
                modifier = Modifier.weight(1f).heightIn(min = ProductTokens.Size.TouchTarget),
            ) {
                Icon(PhosphorIcons.Regular.DownloadSimple, contentDescription = null, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(ProductTokens.Spacing.Lg))
                Text(stringResource(R.string.benchmark_save_short))
            }
        }
        TextButton(onClick = onRun, enabled = enabled) {
            Icon(PhosphorIcons.Regular.ArrowClockwise, contentDescription = null, modifier = Modifier.size(20.dp))
            Spacer(Modifier.width(ProductTokens.Spacing.Lg))
            Text(stringResource(R.string.benchmark_repeat))
        }
    }
}

internal fun speedExplanation(speed: String): String {
    val multiplier = speed.removePrefix("×")
    return "Обрабатывает речь в $multiplier раз быстрее реального времени"
}
