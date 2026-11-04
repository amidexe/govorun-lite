package com.govorun.lite.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ArrowUpRight
import com.govorun.lite.R
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard

@Composable
internal fun EchoProductCard(onClick: () -> Unit) {
    Card(
        onClick = onClick,
        modifier = Modifier.productFeedCard().testTag("echo_product_card"),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Image(
            painterResource(R.drawable.echo_product_cover),
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxWidth().aspectRatio(2.8f),
        )
        Column(
            modifier = Modifier.padding(ProductTokens.Spacing.Section),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Image(
                    painterResource(R.drawable.echo_product_artwork),
                    contentDescription = null,
                    modifier = Modifier.size(44.dp),
                )
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        stringResource(R.string.echo_product_title),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text(
                        stringResource(R.string.echo_product_companion),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Icon(
                    PhosphorIcons.Regular.ArrowUpRight,
                    contentDescription = null,
                    modifier = Modifier.size(20.dp),
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
            Text(
                stringResource(R.string.echo_product_description),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
