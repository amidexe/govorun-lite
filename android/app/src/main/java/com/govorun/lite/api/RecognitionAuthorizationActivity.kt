package com.govorun.lite.api

import android.graphics.drawable.Drawable
import android.os.Bundle
import android.view.WindowManager
import android.widget.ImageView
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductTokens

class RecognitionAuthorizationActivity : ComponentActivity() {
    private lateinit var store: RecognitionAuthorizationStore
    private lateinit var nonce: String
    private var request: AuthorizationRequest? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        window.setHideOverlayWindows(true)
        store = RecognitionAuthorizationStore(this)
        nonce = intent.getStringExtra(EXTRA_NONCE).orEmpty()
        request = resolveRequest()
        setContent {
            GovorunTheme {
                request?.let {
                    AuthorizationDialog(
                        request = it,
                        onDismiss = ::cancel,
                        onApprove = ::approve,
                    )
                } ?: ExpiredRequestDialog(onClose = ::finish)
            }
        }
    }

    private fun resolveRequest(): AuthorizationRequest? {
        val pending = store.pending(nonce) ?: return null
        val identity = CallerIdentityResolver(this).resolveInstalled(pending.packageName) ?: return null
        if (identity.uid != pending.uid || identity.currentSigners != pending.currentSigners) return null
        return AuthorizationRequest(
            pending = pending,
            identity = identity,
            icon = runCatching {
                packageManager.getApplicationIcon(identity.packageName)
            }.getOrNull(),
        )
    }

    private fun cancel() {
        store.consumePending(nonce)
        finish()
    }

    private fun approve() {
        val original = request ?: return finish()
        val consumed = store.consumePending(nonce)
        val current = CallerIdentityResolver(this).resolveInstalled(original.identity.packageName)
        if (
            consumed != null && current != null &&
            current.uid == original.pending.uid &&
            current.currentSigners == original.pending.currentSigners
        ) {
            store.approve(current)
        }
        finish()
    }

    companion object {
        const val EXTRA_NONCE = "recognition_api_nonce"
    }
}

private data class AuthorizationRequest(
    val pending: PendingApproval,
    val identity: CallerIdentity,
    val icon: Drawable?,
)

@Composable
private fun ExpiredRequestDialog(onClose: () -> Unit) {
    AlertDialog(
        onDismissRequest = onClose,
        title = { Text("Запрос устарел") },
        text = { Text("Вернитесь в приложение и запросите доступ ещё раз.") },
        confirmButton = {
            TextButton(onClick = onClose) {
                Text("Закрыть")
            }
        },
    )
}

@Composable
private fun AuthorizationDialog(
    request: AuthorizationRequest,
    onDismiss: () -> Unit,
    onApprove: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Разрешить распознавание?") },
        text = {
            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    AndroidView(
                        factory = { ImageView(it) },
                        update = { it.setImageDrawable(request.icon) },
                        modifier = Modifier.size(40.dp),
                    )
                    Spacer(Modifier.width(ProductTokens.Spacing.Xxl))
                    Column {
                        Text(
                            request.identity.label,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Text(
                            request.identity.packageName,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                Text(
                    text = "${request.identity.label} передаст голос в «Говорун» и получит " +
                        "распознанный текст. Аудио обрабатывается на устройстве, не сохраняется " +
                        "и не отправляется в интернет.",
                    modifier = Modifier.padding(top = ProductTokens.Spacing.Screen),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = "Разрешение относится только к локальному распознаванию. Оно не даёт " +
                        "доступ к вашему аккаунту, подписке или онлайн-стилизации.",
                    modifier = Modifier.padding(top = ProductTokens.Spacing.Md),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Отмена")
            }
        },
        confirmButton = {
            TextButton(onClick = onApprove) {
                Text("Разрешить")
            }
        },
    )
}
