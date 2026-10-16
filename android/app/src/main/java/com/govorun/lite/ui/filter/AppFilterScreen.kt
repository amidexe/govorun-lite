package com.govorun.lite.ui.filter

import android.graphics.drawable.Drawable
import android.widget.ImageView
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.unit.dp
import com.govorun.lite.R
import com.govorun.lite.util.Prefs
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.ProductDivider
import com.product.ui.SettingsSection
import com.product.ui.productScreenPadding

data class FilterApp(
    val label: String,
    val packageName: String,
    val icon: Drawable?,
    val checked: Boolean,
)

data class AppRuleItem(
    val label: String,
    val packageName: String,
    val icon: Drawable?,
    val value: String,
)

@Composable
fun AppFilterScreen(
    apps: List<FilterApp>,
    mode: String,
    onBack: () -> Unit,
    onMode: (String) -> Unit,
    onToggle: (String) -> Unit,
) {
    var query by rememberSaveable { mutableStateOf("") }
    val filtered = apps.filter {
        query.isBlank() ||
            it.label.contains(query, ignoreCase = true) ||
            it.packageName.contains(query, ignoreCase = true)
    }
    ProductScaffold(
        title = androidx.compose.ui.res.stringResource(R.string.title_app_filter),
        onBack = onBack,
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.Top,
        ) {
            item {
                Column(modifier = Modifier.padding(bottom = ProductTokens.Spacing.Md)) {
                    SettingsSection("Правило показа") {
                        FilterRuleRow(
                            title = androidx.compose.ui.res.stringResource(R.string.filter_blacklist),
                            summary = androidx.compose.ui.res.stringResource(
                                R.string.app_filter_desc_blacklist,
                            ),
                            selected = mode == Prefs.APP_FILTER_BLACKLIST,
                            onClick = { onMode(Prefs.APP_FILTER_BLACKLIST) },
                        )
                        ProductDivider()
                        FilterRuleRow(
                            title = androidx.compose.ui.res.stringResource(R.string.filter_whitelist),
                            summary = androidx.compose.ui.res.stringResource(
                                R.string.app_filter_desc_whitelist,
                            ),
                            selected = mode == Prefs.APP_FILTER_WHITELIST,
                            onClick = { onMode(Prefs.APP_FILTER_WHITELIST) },
                        )
                    }
                }
            }
            item {
                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    label = {
                        Text(androidx.compose.ui.res.stringResource(R.string.hint_search_app))
                    },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(
                            start = ProductTokens.Spacing.Sm,
                            end = ProductTokens.Spacing.Sm,
                            bottom = ProductTokens.Spacing.Md,
                        ),
                )
            }
            itemsIndexed(
                items = filtered,
                key = { _, item -> item.packageName },
            ) { index, item ->
                AppRow(
                    item = item,
                    first = index == 0,
                    last = index == filtered.lastIndex,
                    onClick = { onToggle(item.packageName) },
                )
            }
        }
    }
}

@Composable
fun AppRulesScreen(
    apps: List<AppRuleItem>,
    onBack: () -> Unit,
    onSelect: (AppRuleItem) -> Unit,
) {
    var query by rememberSaveable { mutableStateOf("") }
    val filtered = filterAppRules(apps, query)
    ProductScaffold(title = "Правила приложений", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
        ) {
            item {
                Text(
                    "Для каждого приложения можно выбрать стиль, эмодзи и пунктуацию.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(ProductTokens.Spacing.Section),
                )
            }
            item {
                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    label = { Text("Найти приложение") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().padding(
                        start = ProductTokens.Spacing.Sm,
                        end = ProductTokens.Spacing.Sm,
                        bottom = ProductTokens.Spacing.Md,
                    ),
                )
            }
            itemsIndexed(filtered, key = { _, item -> item.packageName }) { index, item ->
                AppRuleRow(
                    item = item,
                    first = index == 0,
                    last = index == filtered.lastIndex,
                    onClick = { onSelect(item) },
                )
            }
        }
    }
}

internal fun filterAppRules(apps: List<AppRuleItem>, query: String): List<AppRuleItem> {
    val normalized = query.trim()
    if (normalized.isEmpty()) return apps
    return apps.filter {
        it.label.contains(normalized, ignoreCase = true) ||
            it.packageName.contains(normalized, ignoreCase = true)
    }
}

@Composable
private fun AppRuleRow(
    item: AppRuleItem,
    first: Boolean,
    last: Boolean,
    onClick: () -> Unit,
) {
    val top = if (first) ProductTokens.Radius.Card else 0.dp
    val bottom = if (last) ProductTokens.Radius.Card else 0.dp
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = ProductTokens.Spacing.Sm)
            .clip(RoundedCornerShape(top, top, bottom, bottom))
            .background(MaterialTheme.colorScheme.surfaceContainerLow)
            .clickable(onClick = onClick)
            .heightIn(min = ProductTokens.Size.Row)
            .padding(ProductTokens.Spacing.Section),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        AndroidView(
            factory = { context ->
                ImageView(context).apply { scaleType = ImageView.ScaleType.CENTER_CROP }
            },
            update = { it.setImageDrawable(item.icon) },
            modifier = Modifier.size(32.dp),
        )
        Spacer(Modifier.width(ProductTokens.Spacing.Lg))
        Column(Modifier.weight(1f)) {
            Text(item.label, style = MaterialTheme.typography.bodyMedium)
            Text(
                item.packageName,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(Modifier.width(ProductTokens.Spacing.Md))
        Text(
            item.value,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun FilterRuleRow(
    title: String,
    summary: String,
    selected: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Md,
            ),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
    ) {
        RadioButton(selected = selected, onClick = onClick)
        Column(modifier = Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyMedium)
            Text(
                summary,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun AppRow(
    item: FilterApp,
    first: Boolean,
    last: Boolean,
    onClick: () -> Unit,
) {
    val top = if (first) ProductTokens.Radius.Card else 0.dp
    val bottom = if (last) ProductTokens.Radius.Card else 0.dp
    val shape = RoundedCornerShape(top, top, bottom, bottom)
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = ProductTokens.Spacing.Sm)
            .clip(shape)
            .background(MaterialTheme.colorScheme.surfaceContainerLow)
            .clickable(onClick = onClick)
            .heightIn(min = ProductTokens.Size.Row)
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Md,
            ),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        AndroidView(
            factory = { context ->
                ImageView(context).apply { scaleType = ImageView.ScaleType.CENTER_CROP }
            },
            update = { it.setImageDrawable(item.icon) },
            modifier = Modifier.size(32.dp),
        )
        Spacer(Modifier.width(ProductTokens.Spacing.Lg))
        androidx.compose.foundation.layout.Column(modifier = Modifier.weight(1f)) {
            Text(item.label, style = MaterialTheme.typography.bodyMedium)
            Text(
                item.packageName,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Checkbox(checked = item.checked, onCheckedChange = { onClick() })
    }
}
