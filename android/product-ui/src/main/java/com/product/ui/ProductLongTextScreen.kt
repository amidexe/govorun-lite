package com.product.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.foundation.lazy.LazyColumn

@Composable
fun ProductLongTextScreen(
    title: String,
    body: String,
    onBack: () -> Unit,
    intro: String? = null,
    introSectionTitle: String = "Коротко",
    bodySectionTitle: String = "Подробности",
) {
    ProductScaffold(title = title, onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            intro?.let { text ->
                item {
                    SettingsSection(introSectionTitle) {
                        SelectionContainer {
                            Text(
                                text,
                                modifier = Modifier.padding(ProductTokens.Spacing.Section),
                                style = MaterialTheme.typography.bodyMedium,
                            )
                        }
                    }
                }
            }
            item {
                SettingsSection(bodySectionTitle) {
                    SelectionContainer {
                        Text(
                            body,
                            modifier = Modifier.padding(ProductTokens.Spacing.Section),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}
