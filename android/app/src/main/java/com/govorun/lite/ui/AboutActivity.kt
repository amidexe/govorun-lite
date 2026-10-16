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
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Duotone
import com.adamglin.phosphoricons.duotone.ClockClockwise
import com.adamglin.phosphoricons.duotone.Code
import com.adamglin.phosphoricons.duotone.GithubLogo
import com.adamglin.phosphoricons.duotone.Gavel
import com.adamglin.phosphoricons.duotone.ShieldCheck
import com.adamglin.phosphoricons.duotone.Heart
import com.adamglin.phosphoricons.duotone.Star
import com.adamglin.phosphoricons.duotone.User
import com.govorun.lite.BuildConfig
import com.govorun.lite.R
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.FeedCard
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.SettingsInfoRow
import com.product.ui.SettingsLinkRow
import com.product.ui.SettingsSection
import com.product.ui.productScreenPadding

class AboutActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            GovorunTheme {
                AboutScreen(
                    onBack = ::finish,
                    onGithub = { openExternal(getString(R.string.about_github_url)) },
                    onLicenses = {
                        startActivity(Intent(this, LicensesActivity::class.java))
                    },
                    onPrivacy = { openExternal(getString(R.string.about_privacy_url)) },
                    onReview = { openExternal(getString(R.string.main_pro_review_url)) },
                )
            }
        }
    }

    private fun openExternal(url: String) {
        try {
            startActivity(
                Intent(Intent.ACTION_VIEW, Uri.parse(url))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
        } catch (_: ActivityNotFoundException) {
        }
    }
}

@Composable
private fun AboutScreen(
    onBack: () -> Unit,
    onGithub: () -> Unit,
    onLicenses: () -> Unit,
    onPrivacy: () -> Unit,
    onReview: () -> Unit,
) {
    ProductScaffold(title = "О Говоруне", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                FeedCard {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(ProductTokens.Spacing.Section),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
                    ) {
                        Icon(
                            ImageVector.vectorResource(R.drawable.logo_bird),
                            contentDescription = "Логотип Говоруна",
                            modifier = Modifier.size(56.dp),
                            tint = MaterialTheme.colorScheme.primary,
                        )
                        Text("Говорун", style = MaterialTheme.typography.titleLarge)
                        Text(
                            AppBuildIdentity.versionLine(
                                BuildConfig.VERSION_NAME,
                                BuildConfig.VERSION_CODE,
                            ),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            AppBuildIdentity.buildLine(BuildConfig.BUILD_ID),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            item {
                SettingsSection("О приложении") {
                    SettingsInfoRow(
                        ImageVector.vectorResource(R.drawable.ic_bird_24),
                        stringResource(R.string.about_row_engine_title),
                        stringResource(R.string.about_row_engine_body),
                    )
                    ProductDivider()
                    SettingsInfoRow(
                        PhosphorIcons.Duotone.ShieldCheck,
                        stringResource(R.string.about_row_offline_title),
                        stringResource(R.string.about_row_offline_body),
                    )
                    ProductDivider()
                    SettingsInfoRow(
                        PhosphorIcons.Duotone.Code,
                        stringResource(R.string.about_row_source_title),
                        stringResource(R.string.about_row_source_body),
                    )
                    ProductDivider()
                    SettingsInfoRow(
                        PhosphorIcons.Duotone.ClockClockwise,
                        stringResource(R.string.about_row_updates_title),
                        stringResource(R.string.about_row_updates_body),
                    )
                }
            }
            item {
                SettingsSection("Автор и благодарности") {
                    SettingsInfoRow(
                        PhosphorIcons.Duotone.User,
                        stringResource(R.string.about_author_title),
                        stringResource(R.string.about_author_body),
                    )
                    ProductDivider()
                    SettingsInfoRow(
                        PhosphorIcons.Duotone.Heart,
                        stringResource(R.string.about_thanks_title),
                        stringResource(R.string.about_thanks_body),
                    )
                    ProductDivider()
                    SettingsLinkRow(
                        PhosphorIcons.Duotone.Star,
                        "Оставить отзыв",
                        "Отзывы помогают Говоруну развиваться",
                        onClick = onReview,
                    )
                }
            }
            item {
                SettingsSection("Документы и ссылки") {
                    SettingsLinkRow(
                        PhosphorIcons.Duotone.GithubLogo,
                        stringResource(R.string.about_action_github_title),
                        stringResource(R.string.about_action_github_sub),
                        onClick = onGithub,
                    )
                    ProductDivider()
                    SettingsLinkRow(
                        PhosphorIcons.Duotone.Gavel,
                        stringResource(R.string.about_action_licenses_title),
                        stringResource(R.string.about_action_licenses_sub),
                        onClick = onLicenses,
                    )
                    ProductDivider()
                    SettingsLinkRow(
                        PhosphorIcons.Duotone.ShieldCheck,
                        stringResource(R.string.about_action_privacy_title),
                        stringResource(R.string.about_action_privacy_sub),
                        onClick = onPrivacy,
                    )
                }
            }
            item {
                Text(
                    stringResource(R.string.about_footer_fmt, BuildConfig.VERSION_NAME),
                    modifier = Modifier.padding(ProductTokens.Spacing.Section),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
