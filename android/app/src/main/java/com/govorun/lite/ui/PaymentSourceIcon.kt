package com.govorun.lite.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CreditCard
import com.govorun.lite.R

@Composable
internal fun PaymentSourceIcon(kind: String) {
    if (kind == "sbp") {
        Image(painterResource(R.drawable.ic_payment_sbp), null, Modifier.size(28.dp))
    } else {
        Icon(PhosphorIcons.Regular.CreditCard, null, Modifier.size(28.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
