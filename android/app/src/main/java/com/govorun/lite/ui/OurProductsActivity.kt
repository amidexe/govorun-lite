package com.govorun.lite.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.vectorResource
import com.govorun.lite.R
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.SettingsInfoRow
import com.product.ui.SettingsSection
import com.product.ui.productScreenPadding

class OurProductsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            GovorunTheme {
                OurProductsScreen(onBack = ::finish)
            }
        }
    }
}

@Composable
private fun OurProductsScreen(onBack: () -> Unit) {
    ProductScaffold(title = "Наши продукты", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                SettingsSection("Приложения") {
                    SettingsInfoRow(
                        ImageVector.vectorResource(R.drawable.ic_bird_24),
                        "Говорун",
                        "Голосовой ввод на русском: быстро, приватно и без интернета",
                    )
                }
            }
            item {
                Text(
                    "Здесь собраны выпущенные приложения автора. " +
                        "Новые продукты появятся после официального выхода.",
                    modifier = Modifier.padding(ProductTokens.Spacing.Section),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
