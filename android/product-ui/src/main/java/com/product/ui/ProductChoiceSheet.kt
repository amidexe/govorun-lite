package com.product.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Check

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun <T> ProductChoiceSheet(
    title: String,
    selected: T,
    options: List<T>,
    optionLabel: (T) -> String,
    onSelected: (T) -> Unit,
    onDismissRequest: () -> Unit,
) {
    ModalBottomSheet(
        onDismissRequest = onDismissRequest,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(max = 640.dp)
                .verticalScroll(rememberScrollState())
                .padding(
                    start = ProductTokens.Spacing.Screen,
                    end = ProductTokens.Spacing.Screen,
                    bottom = ProductTokens.Spacing.Screen,
                ),
        ) {
            Text(
                title,
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(
                    horizontal = ProductTokens.Spacing.Section,
                    vertical = ProductTokens.Spacing.Xxl,
                ),
            )
            FeedCard(modifier = Modifier.fillMaxWidth()) {
                Column {
                    options.forEachIndexed { index, option ->
                        ProductChoiceRow(
                            title = optionLabel(option),
                            selected = option == selected,
                            onClick = {
                                onSelected(option)
                                onDismissRequest()
                            },
                        )
                        if (index < options.lastIndex) ProductDivider()
                    }
                }
            }
        }
    }
}

@Composable
private fun ProductChoiceRow(
    title: String,
    selected: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = ProductTokens.Size.Row)
            .background(
                if (selected) {
                    MaterialTheme.colorScheme.secondaryContainer.copy(
                        alpha = ProductTokens.Alpha.SelectedSurface,
                    )
                } else {
                    Color.Transparent
                },
            )
            .clickable(onClick = onClick)
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Md,
            ),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl),
    ) {
        Text(
            title,
            modifier = Modifier.weight(1f),
            style = MaterialTheme.typography.bodyMedium,
        )
        if (selected) {
            Icon(
                PhosphorIcons.Regular.Check,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                tint = MaterialTheme.colorScheme.primary,
            )
        } else {
            Spacer(Modifier.size(ProductTokens.Size.ActiveIcon))
        }
    }
}
