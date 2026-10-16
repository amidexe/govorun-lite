package com.govorun.lite.ui.benchmark

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.rememberVectorPainter
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Cpu
import com.adamglin.phosphoricons.regular.ShieldCheck
import com.govorun.lite.R
import com.product.ui.ProductTokens

internal const val BENCHMARK_SHARE_BACKGROUND_ARGB: Long = 0xFFFBFAF7

private object ShareCardColors {
    val background = androidx.compose.ui.graphics.Color(BENCHMARK_SHARE_BACKGROUND_ARGB)
    val content = ProductTokens.Color.LightText
    val mutedContent = ProductTokens.Color.LightMutedText
    val mutedSurface = ProductTokens.Color.LightMutedSurface
    val accent = ProductTokens.Color.BrandAccent
    val divider = ProductTokens.Color.LightMutedText.copy(alpha = 0.24f)
}

@Composable
internal fun BenchmarkResultCard(state: BenchmarkUiState, modifier: Modifier = Modifier) {
    Card(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = ShareCardColors.background,
            contentColor = ShareCardColors.content,
        ),
    ) {
        Column(
            modifier = Modifier.padding(ProductTokens.Spacing.Screen),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
        ) {
            ResultBrand()
            Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm)) {
                Text(
                    text = state.device,
                    style = MaterialTheme.typography.titleMedium,
                    color = ShareCardColors.content,
                )
                Text(
                    text = state.system,
                    style = MaterialTheme.typography.bodySmall,
                    color = ShareCardColors.mutedContent,
                )
            }
            ResultHero(state)
            AcceleratorBadge(state.provider)
            HorizontalDivider(color = ShareCardColors.divider)
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                ResultMetric(
                    label = stringResource(R.string.benchmark_cold_title),
                    value = state.coldTime,
                    modifier = Modifier.weight(1f),
                )
                ResultMetric(
                    label = stringResource(R.string.benchmark_warm_title),
                    value = state.warmTime,
                    modifier = Modifier.weight(1f),
                )
                ResultMetric(
                    label = stringResource(R.string.benchmark_long_metric_title),
                    value = state.longTime,
                    modifier = Modifier.weight(1f),
                )
            }
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
            ) {
                Icon(
                    PhosphorIcons.Regular.ShieldCheck,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                    tint = ShareCardColors.accent,
                )
                Text(
                    text = stringResource(R.string.benchmark_local_only_confirmation),
                    style = MaterialTheme.typography.bodySmall,
                    color = ShareCardColors.mutedContent,
                )
            }
        }
    }
}

@Composable
private fun ResultBrand() {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
    ) {
        Icon(
            painter = rememberVectorPainter(ImageVector.vectorResource(R.drawable.logo_bird)),
            contentDescription = null,
            modifier = Modifier.size(28.dp),
            tint = ShareCardColors.accent,
        )
        Column {
            Text(
                stringResource(R.string.benchmark_share_card_brand),
                style = MaterialTheme.typography.titleMedium,
                color = ShareCardColors.content,
            )
            Text(
                stringResource(R.string.benchmark_share_card_subtitle),
                style = MaterialTheme.typography.labelSmall,
                color = ShareCardColors.mutedContent,
            )
        }
    }
}

@Composable
private fun ResultHero(state: BenchmarkUiState) {
    Column(
        modifier = Modifier.fillMaxWidth()
            .background(ShareCardColors.mutedSurface, RoundedCornerShape(ProductTokens.Radius.Md))
            .padding(ProductTokens.Spacing.Section),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
    ) {
        Text(
            text = state.speed.orEmpty(),
            style = MaterialTheme.typography.displayLarge,
            color = ShareCardColors.accent,
        )
        Text(
            text = state.rating ?: stringResource(R.string.benchmark_result_default),
            style = MaterialTheme.typography.titleLarge,
            color = ShareCardColors.content,
            textAlign = TextAlign.Center,
        )
        Text(
            text = speedExplanation(state.speed.orEmpty()),
            style = MaterialTheme.typography.bodySmall,
            color = ShareCardColors.mutedContent,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
private fun AcceleratorBadge(provider: String) {
    val acceleratorLabel = stringResource(R.string.benchmark_label_provider)
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
        modifier = Modifier.semantics {
            contentDescription = "$acceleratorLabel: $provider"
        },
    ) {
        Icon(PhosphorIcons.Regular.Cpu, contentDescription = null,
            modifier = Modifier.size(18.dp), tint = ShareCardColors.mutedContent)
        Text(
            text = stringResource(R.string.benchmark_accelerator_value, provider),
            style = MaterialTheme.typography.labelMedium,
            color = ShareCardColors.mutedContent,
        )
    }
}

@Composable
private fun ResultMetric(label: String, value: String, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs),
    ) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleMedium,
            color = ShareCardColors.content,
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = ShareCardColors.mutedContent,
        )
    }
}

