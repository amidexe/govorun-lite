package com.govorun.lite.ui

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Duotone
import com.adamglin.phosphoricons.duotone.ShareNetwork
import com.adamglin.phosphoricons.duotone.Star
import com.govorun.lite.R
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.FeedCard
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.SettingsLinkRow
import com.product.ui.SettingsSection
import com.product.ui.productScreenPadding

/**
 * Kept under its historical component name so old shortcuts keep working.
 * The screen is now the permanent project-support destination, not a list of
 * promised or already-shipped features.
 */
class FuturePlansActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            GovorunTheme {
                ReviewScreen(
                    onBack = ::finish,
                    onReview = ::openRuStoreReview,
                    onShare = ::shareApp,
                )
            }
        }
    }

    private fun openRuStoreReview() {
        try {
            startActivity(
                Intent(Intent.ACTION_VIEW, Uri.parse(getString(R.string.main_pro_review_url)))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
        } catch (_: ActivityNotFoundException) {
        }
    }

    private fun shareApp() {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, getString(R.string.main_share_app_text))
        }
        runCatching {
            startActivity(Intent.createChooser(intent, getString(R.string.main_share_chooser)))
        }
    }
}

@Composable
private fun ReviewScreen(
    onBack: () -> Unit,
    onReview: () -> Unit,
    onShare: () -> Unit,
) {
    ProductScaffold(title = "Поддержать Говорун", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                FeedCard {
                    Column(
                        modifier = Modifier.padding(ProductTokens.Spacing.Screen),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
                    ) {
                        Icon(
                            PhosphorIcons.Duotone.Star,
                            contentDescription = null,
                            modifier = Modifier.size(48.dp),
                            tint = MaterialTheme.colorScheme.primary,
                        )
                        Text(
                            "Ваш отзыв помогает Говоруну расти",
                            style = MaterialTheme.typography.titleLarge,
                        )
                        Text(
                            "Если приложение экономит вам время, расскажите об этом в RuStore. " +
                                "Отзывы помогают другим людям найти Говоруна и подсказывают, " +
                                "что действительно стоит улучшать.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            item {
                SettingsSection("Помочь проекту") {
                    SettingsLinkRow(
                        PhosphorIcons.Duotone.Star,
                        "Оставить отзыв в RuStore",
                        "Это займёт пару минут и очень поможет",
                        onClick = onReview,
                    )
                    ProductDivider()
                    SettingsLinkRow(
                        PhosphorIcons.Duotone.ShareNetwork,
                        "Рассказать о Говоруне",
                        "Поделиться приложением с друзьями",
                        onClick = onShare,
                    )
                }
            }
        }
    }
}
