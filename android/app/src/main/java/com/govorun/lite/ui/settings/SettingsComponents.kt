package com.govorun.lite.ui.settings

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.vector.ImageVector
import com.product.ui.ProductDivider
import com.product.ui.ProductChoiceSheet
import com.product.ui.SettingsLinkRow as ProductSettingsLinkRow
import com.product.ui.SettingsSection as ProductSettingsSection
import com.product.ui.SettingsSummaryRow as ProductSettingsSummaryRow
import com.product.ui.SettingsSwitchRow as ProductSettingsSwitchRow

@Composable
internal fun SettingsSection(
    title: String,
    content: @Composable () -> Unit,
) = ProductSettingsSection(title, content)

@Composable
internal fun SettingsLinkRow(
    icon: ImageVector? = null,
    title: String,
    summary: String? = null,
    value: String? = null,
    enabled: Boolean = true,
    onClick: () -> Unit,
) = ProductSettingsLinkRow(icon, title, summary, value, enabled, onClick)

@Composable
internal fun SettingsSwitchRow(
    title: String,
    summary: String? = null,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) = ProductSettingsSwitchRow(title, summary, checked, onCheckedChange)

@Composable
internal fun SettingsSummaryRow(
    title: String,
    value: String,
) = ProductSettingsSummaryRow(title, value)

@Composable
internal fun <T> SettingsChoiceSheet(
    title: String,
    selected: T,
    options: List<T>,
    optionLabel: (T) -> String,
    onSelected: (T) -> Unit,
    onDismissRequest: () -> Unit,
) = ProductChoiceSheet(
    title = title,
    selected = selected,
    options = options,
    optionLabel = optionLabel,
    onSelected = onSelected,
    onDismissRequest = onDismissRequest,
)

@Composable
internal fun SettingsDivider() = ProductDivider()
