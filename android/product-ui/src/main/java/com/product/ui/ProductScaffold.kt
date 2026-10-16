package com.product.ui

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ArrowLeft

data class ProductTopAction(
    val icon: ImageVector,
    val description: String,
    val onClick: () -> Unit,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProductScaffold(
    title: String,
    modifier: Modifier = Modifier,
    onBack: (() -> Unit)? = null,
    navigationAction: ProductTopAction? = null,
    actions: List<ProductTopAction> = emptyList(),
    content: @Composable (PaddingValues) -> Unit,
) {
    Scaffold(
        modifier = modifier.fillMaxSize(),
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                navigationIcon = {
                    if (onBack != null) {
                        IconButton(onClick = onBack) {
                            Icon(
                                PhosphorIcons.Regular.ArrowLeft,
                                contentDescription = "Назад",
                                modifier = Modifier.size(ProductTokens.Size.TopBarNavigationIcon),
                            )
                        }
                    } else if (navigationAction != null) {
                        IconButton(onClick = navigationAction.onClick) {
                            Icon(
                                navigationAction.icon,
                                contentDescription = navigationAction.description,
                                modifier = Modifier.size(ProductTokens.Size.TopBarNavigationIcon),
                            )
                        }
                    }
                },
                title = { Text(title) },
                actions = {
                    actions.forEach { action ->
                        IconButton(onClick = action.onClick) {
                            Icon(
                                action.icon,
                                contentDescription = action.description,
                                modifier = Modifier.size(ProductTokens.Size.TopBarActionIcon),
                            )
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
            )
        },
        content = content,
    )
}

fun productScreenPadding(
    top: androidx.compose.ui.unit.Dp = ProductTokens.Spacing.Section,
    bottom: androidx.compose.ui.unit.Dp = ProductTokens.Spacing.Screen,
) = PaddingValues(
    start = ProductTokens.Spacing.Section,
    end = ProductTokens.Spacing.Xxl,
    top = top,
    bottom = bottom,
)
