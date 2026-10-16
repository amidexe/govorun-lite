package com.product.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CaretDown
import com.adamglin.phosphoricons.regular.CaretRight

@Composable
fun FeedCard(
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Card(
        modifier = modifier.productFeedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        content()
    }
}

fun Modifier.productFeedCard(): Modifier = fillMaxWidth().padding(
    horizontal = ProductTokens.Spacing.Sm,
    vertical = ProductTokens.Spacing.Xs,
)

@Composable
fun FeedSectionHeader(
    title: String,
    icon: ImageVector? = null,
    trailingText: String? = null,
    count: Int? = null,
    expanded: Boolean? = null,
    onToggle: (() -> Unit)? = null,
    horizontalPadding: Dp = ProductTokens.Spacing.Lg,
    trailingTextColor: Color = MaterialTheme.colorScheme.onSurfaceVariant,
    actions: @Composable RowScope.() -> Unit = {},
) {
    val interaction = remember { MutableInteractionSource() }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = ProductTokens.Size.Row)
            .then(
                if (onToggle != null) {
                    Modifier.clickable(
                        interactionSource = interaction,
                        indication = null,
                        onClick = onToggle,
                    )
                } else {
                    Modifier
                },
            )
            .padding(horizontal = horizontalPadding),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
    ) {
        expanded?.let {
            Icon(
                if (it) PhosphorIcons.Regular.CaretDown else PhosphorIcons.Regular.CaretRight,
                contentDescription = if (it) "Свернуть" else "Развернуть",
                modifier = Modifier.size(ProductTokens.Size.SlotIcon),
                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(
                    alpha = ProductTokens.Alpha.MutedIcon,
                ),
            )
        }
        icon?.let {
            Icon(
                it,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.SlotIcon),
                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(
                    alpha = ProductTokens.Alpha.MutedIcon,
                ),
            )
        }
        Text(
            title,
            modifier = Modifier.weight(1f),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        trailingText?.let {
            Text(
                it,
                style = MaterialTheme.typography.labelSmall,
                color = trailingTextColor,
            )
        }
        count?.let {
            Text(
                it.toString(),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(
                    alpha = ProductTokens.Alpha.MutedIcon,
                ),
            )
        }
        actions()
    }
}

@Composable
fun ProductDivider() {
    HorizontalDivider(
        modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Section),
        color = MaterialTheme.colorScheme.outlineVariant.copy(
            alpha = ProductTokens.Alpha.Divider,
        ),
    )
}
