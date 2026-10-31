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
import com.product.ui.SettingsSwitchRow
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
                    onVolumeKey = ::setVolumeKey,
                    onSmart = ::setSmart,
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
        val store = RecognitionAuthorizationStore(this)
        apps = store.approvedPackages().map { packageName ->
            val identity = resolver.resolveInstalled(packageName)
            ApprovedApp(
                label = identity?.label ?: packageName,
                packageName = packageName,
                icon = runCatching {
                    packageManager.getApplicationIcon(packageName)
                }.getOrNull(),
                recognition = store.scopes(packageName) and RecognitionApi.SCOPE_LOCAL_RECOGNITION != 0,
                echo = identity?.let(com.govorun.lite.api.smart.EchoApp::matches) == true,
                volumeKey = store.scopes(packageName) and RecognitionApi.SCOPE_VOLUME_KEY != 0,
                smart = store.scopes(packageName) and RecognitionApi.SCOPE_SMART_FUNCTIONS != 0,
            )
        }
    }

    private fun setVolumeKey(packageName: String, enabled: Boolean) {
        RecognitionApiControl.setVolumeKey(this, packageName, enabled)
        refresh()
    }

    private fun setSmart(packageName: String, enabled: Boolean) {
        RecognitionApiControl.setSmartFunctions(this, packageName, enabled)
        refresh()
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
    val recognition: Boolean = true,
    /** Говорун.Эхо: the one app that may record by the volume key on the lock screen. */
    val echo: Boolean = false,
    val volumeKey: Boolean = false,
    /** The person once pressed a smart function in the app, so its own tasks (итоги of a new note) go by themselves. */
    val smart: Boolean = false,
)

@Composable
private fun AuthorizedAppsScreen(
    apps: List<ApprovedApp>,
    onBack: () -> Unit,
    onRevoke: (String) -> Unit,
    onVolumeKey: (String, Boolean) -> Unit,
    onSmart: (String, Boolean) -> Unit,
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
                        summary = "Приложения получают только распознавание на устройстве: без аккаунта, подписки и онлайн-стилизации. Собственная запись Говоруна всегда имеет приоритет.",
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
                            if (app.echo) {
                                SettingsSwitchRow(
                                    title = "Запись кнопкой «−» на заблокированном экране",
                                    summary = "Держите «−», пока экран заблокирован, — Эхо записывает заметку; отпустили — сохранило",
                                    checked = app.volumeKey,
                                    onCheckedChange = { onVolumeKey(app.packageName, it) },
                                )
                                SettingsSwitchRow(
                                    title = "Умные функции сами",
                                    summary = "Итоги новых заметок через аккаунт Говоруна. Включается после первого нажатия «Итоги» или «Улучшить» в Эхе",
                                    checked = app.smart,
                                    onCheckedChange = { onSmart(app.packageName, it) },
                                )
                            }
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
            if (app.recognition) Text(
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
