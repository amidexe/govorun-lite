package com.govorun.lite.api

import android.graphics.drawable.Drawable
import android.os.Bundle
import android.widget.ImageView
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.govorun.lite.R
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.SettingsInfoRow
import com.product.ui.SettingsSection
import com.product.ui.productScreenPadding

class AuthorizedAppsActivity : ComponentActivity() {
    private var apps by mutableStateOf<List<ApprovedApp>>(emptyList())

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            GovorunTheme {
                AuthorizedAppsScreen(
                    apps = apps,
                    onBack = ::finish,
                    onRevoke = ::revoke,
                )
            }
        }
    }

    override fun onResume() {
        super.onResume()
        refresh()
    }

    private fun refresh() {
        val resolver = CallerIdentityResolver(this)
        apps = RecognitionAuthorizationStore(this).approvedPackages().map { packageName ->
            val identity = resolver.resolveInstalled(packageName)
            ApprovedApp(
                label = identity?.label ?: packageName,
                packageName = packageName,
                icon = runCatching {
                    packageManager.getApplicationIcon(packageName)
                }.getOrNull(),
            )
        }
    }

    private fun revoke(packageName: String) {
        RecognitionApiControl.revoke(this, packageName)
        refresh()
    }
}

private data class ApprovedApp(
    val label: String,
    val packageName: String,
    val icon: Drawable?,
)

@Composable
private fun AuthorizedAppsScreen(
    apps: List<ApprovedApp>,
    onBack: () -> Unit,
    onRevoke: (String) -> Unit,
) {
    ProductScaffold(
        title = "API для приложений",
        onBack = onBack,
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                SettingsSection("Для разработчиков") {
                    SettingsInfoRow(
                        title = "Recognition API v1",
                        summary = "Подтверждённое вами приложение может передать Говоруну свою запись и получить только результат её локального распознавания. API не получает текст вашей обычной диктовки.",
                    )
                    ProductDivider()
                    SettingsInfoRow(
                        title = "Только локальное распознавание",
                        summary = "Доступ не включает аккаунт, подписку или онлайн-стилизацию. Собственная запись Говоруна всегда имеет приоритет.",
                    )
                }
            }
            item {
                SettingsSection("Доступ к движку Говоруна") {
                    if (apps.isEmpty()) {
                        SettingsInfoRow(
                            title = "Доступ пока никому не предоставлен",
                            summary = "Подключённые приложения появятся здесь после вашего подтверждения.",
                        )
                    } else {
                        apps.forEachIndexed { index, app ->
                            if (index > 0) ProductDivider()
                            ApprovedAppRow(app) { onRevoke(app.packageName) }
                        }
                    }
                }
            }
            item {
                SettingsSection("Безопасность") {
                    SettingsInfoRow(
                        title = "Вы подтверждаете каждое приложение",
                        summary = "Говорун связывает разрешение с пакетом и подписью приложения. Доступ можно отозвать здесь в любой момент.",
                    )
                }
            }
        }
    }
}

@Composable
private fun ApprovedAppRow(
    app: ApprovedApp,
    onRevoke: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = ProductTokens.Size.Row)
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Md,
            ),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        AndroidView(
            factory = { ImageView(it) },
            update = { it.setImageDrawable(app.icon) },
            modifier = Modifier.size(32.dp),
        )
        Spacer(Modifier.width(ProductTokens.Spacing.Lg))
        Column(modifier = Modifier.weight(1f)) {
            Text(app.label, style = MaterialTheme.typography.bodyMedium)
            Text(
                app.packageName,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "Локальное распознавание",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        TextButton(onClick = onRevoke) {
            Text("Отозвать")
        }
    }
}
