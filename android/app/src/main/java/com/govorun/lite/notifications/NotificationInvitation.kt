package com.govorun.lite.notifications

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard

@Composable
fun NotificationInvitation(onAllow: () -> Unit, onLater: () -> Unit) {
    Card(
        modifier = Modifier.productFeedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(Modifier.padding(ProductTokens.Spacing.Section)) {
            Text("Получать уведомления об обновлениях и акциях?", style = MaterialTheme.typography.bodyLarge)
            Row(Modifier.align(Alignment.End)) {
                TextButton(onClick = onLater) { Text("Не сейчас") }
                TextButton(onClick = onAllow) { Text("Разрешить") }
            }
        }
    }
}
