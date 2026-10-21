package com.product.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.Density
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CaretRight

data class ProductSectionItem<T>(
    val key: T,
    val title: String,
    val icon: ImageVector,
)

@Composable
fun <T> ProductSettingsShell(
    title: String,
    items: List<ProductSectionItem<T>>,
    selected: T,
    onSelected: (T) -> Unit,
    onBack: () -> Unit,
    content: LazyListScope.() -> Unit,
) {
    ProductScaffold(title = title, onBack = onBack) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            ProductSectionPicker(items, selected, onSelected)
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = productScreenPadding(),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
                content = content,
            )
        }
    }
}

// Sections spread across the row when they fit. LazyRow arranges only with spare space, so a
// narrow screen or a large font keeps them scrolling with the small gap.
private object SectionPickerArrangement : Arrangement.Horizontal {
    override val spacing = ProductTokens.Spacing.Sm
    override fun Density.arrange(totalSize: Int, sizes: IntArray, layoutDirection: LayoutDirection, outPositions: IntArray) =
        with(Arrangement.SpaceEvenly) { arrange(totalSize, sizes, layoutDirection, outPositions) }
}

@Composable
fun <T> ProductSectionPicker(
    items: List<ProductSectionItem<T>>,
    selected: T,
    onSelected: (T) -> Unit,
) {
    LazyRow(
        horizontalArrangement = SectionPickerArrangement,
        contentPadding = androidx.compose.foundation.layout.PaddingValues(
            horizontal = ProductTokens.Spacing.Sm,
            vertical = ProductTokens.Spacing.Xs,
        ),
        modifier = Modifier.fillMaxWidth(),
    ) {
        items(items.size, key = { items[it].title }) { index ->
            val item = items[index]
            val active = item.key == selected
            val shape = RoundedCornerShape(ProductTokens.Radius.Md)
            Surface(
                color = if (active) {
                    MaterialTheme.colorScheme.surfaceContainerHighest.copy(
                        alpha = ProductTokens.Alpha.SelectedSurface,
                    )
                } else {
                    Color.Transparent
                },
                shape = shape,
                modifier = Modifier
                    .widthIn(min = 76.dp)
                    .defaultMinSize(minHeight = ProductTokens.Size.Row)
                    .clip(shape)
                    .clickable { onSelected(item.key) },
            ) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs),
                    modifier = Modifier.padding(
                        horizontal = ProductTokens.Spacing.Xl,
                        vertical = ProductTokens.Spacing.Md,
                    ),
                ) {
                    Icon(
                        item.icon,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                        tint = if (active) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                    )
                    Text(
                        item.title,
                        style = MaterialTheme.typography.labelLarge,
                        color = if (active) {
                            MaterialTheme.colorScheme.onSurface
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        maxLines = 1,
                    )
                }
            }
        }
    }
}

@Composable
fun SettingsSection(
    title: String,
    content: @Composable () -> Unit,
) {
    FeedCard {
        Column {
            FeedSectionHeader(
                title = title,
                horizontalPadding = ProductTokens.Spacing.Section,
            )
            content()
        }
    }
}

@Composable
fun SettingsLinkRow(
    icon: ImageVector? = null,
    title: String,
    summary: String? = null,
    value: String? = null,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    Surface(onClick = onClick, enabled = enabled, color = Color.Transparent) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .alpha(if (enabled) 1f else 0.38f)
                .heightIn(min = ProductTokens.Size.Row)
                .padding(
                    horizontal = ProductTokens.Spacing.Section,
                    vertical = ProductTokens.Spacing.Md,
                ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            icon?.let {
                Icon(
                    it,
                    contentDescription = null,
                    modifier = Modifier.size(20.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.width(ProductTokens.Spacing.Lg))
            }
            Column(modifier = Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.bodyMedium)
                summary?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            value?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.width(ProductTokens.Spacing.Md))
            }
            Icon(
                PhosphorIcons.Regular.CaretRight,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
fun SettingsInfoRow(
    icon: ImageVector? = null,
    title: String,
    summary: String? = null,
    trailing: @Composable (() -> Unit)? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = ProductTokens.Size.Row)
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Md,
            ),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        icon?.let {
            Icon(
                it,
                contentDescription = null,
                modifier = Modifier.size(20.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.width(ProductTokens.Spacing.Lg))
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyMedium)
            summary?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        trailing?.invoke()
    }
}

@Composable
fun SettingsSummaryRow(
    title: String,
    value: String,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = ProductTokens.Size.Row)
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Md,
            ),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
    ) {
        Text(
            title,
            modifier = Modifier.weight(1f),
            style = MaterialTheme.typography.bodyMedium,
        )
        Text(
            value,
            modifier = Modifier.widthIn(max = 180.dp),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
fun SettingsSwitchRow(
    title: String,
    summary: String? = null,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    enabled: Boolean = true,
) {
    Surface(color = Color.Transparent) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = ProductTokens.Size.Row)
                .alpha(if (enabled) 1f else 0.55f)
                .toggleable(
                    value = checked,
                    enabled = enabled,
                    role = Role.Switch,
                    onValueChange = onCheckedChange,
                )
                .padding(
                    horizontal = ProductTokens.Spacing.Section,
                    vertical = ProductTokens.Spacing.Md,
                ),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl),
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.bodyMedium)
                summary?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            CompactSwitch(checked)
        }
    }
}

@Composable
private fun CompactSwitch(
    checked: Boolean,
) {
    val track = if (checked) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.surfaceContainerHigh
    }
    val thumb = if (checked) {
        MaterialTheme.colorScheme.onPrimary
    } else {
        MaterialTheme.colorScheme.onSurfaceVariant
    }
    Box(
        contentAlignment = if (checked) Alignment.CenterEnd else Alignment.CenterStart,
        modifier = Modifier
            .width(ProductTokens.Size.CompactSwitchWidth)
            .height(ProductTokens.Size.CompactSwitchHeight)
            .clip(CircleShape)
            .background(track)
            .padding(ProductTokens.Spacing.Xs),
    ) {
        Box(
            modifier = Modifier
                .size(ProductTokens.Size.SlotIcon)
                .clip(CircleShape)
                .background(thumb),
        )
    }
}
