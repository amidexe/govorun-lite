package com.govorun.lite.stylization

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import com.govorun.lite.R

@Composable
internal fun StyleBetaNotice(compact: Boolean = false) {
    Text(
        stringResource(if (compact) R.string.stylization_beta_label else R.string.stylization_beta_notice),
        style = if (compact) MaterialTheme.typography.bodySmall else MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
