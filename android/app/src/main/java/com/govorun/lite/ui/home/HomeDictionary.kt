package com.govorun.lite.ui.home

import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.BookOpenText
import com.adamglin.phosphoricons.regular.CaretRight
import com.product.ui.ProductTokens

@Composable
internal fun DictionaryCard(
    state: DictionarySummaryState,
    onEvent: (HomeEvent) -> Unit,
) {
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        FeedSectionHeader(
            icon = PhosphorIcons.Regular.BookOpenText,
            title = "Словарь",
            trailingText = dictionaryStatusLabel(state),
            onToggle = { onEvent(HomeEvent.OpenDictionary) },
        ) {
            NavigationIcon(PhosphorIcons.Regular.CaretRight)
        }
    }
}

@Composable
private fun NavigationIcon(icon: ImageVector) {
    Icon(
        imageVector = icon,
        contentDescription = null,
        modifier = Modifier.size(ProductTokens.Size.SlotIcon),
        tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(
            alpha = ProductTokens.Alpha.MutedIcon,
        ),
    )
}
