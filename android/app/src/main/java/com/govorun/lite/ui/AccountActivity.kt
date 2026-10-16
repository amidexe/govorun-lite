package com.govorun.lite.ui

import android.os.Bundle
import android.content.Intent
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withLink
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.UserCircle
import com.govorun.lite.account.userMessage
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.AccountState
import com.govorun.lite.account.AccountDataSyncMode
import com.govorun.lite.account.AccountLoginReturnHandoff
import com.govorun.lite.account.finishAccountLogin
import com.govorun.lite.account.shouldFinishAccountLogin
import com.govorun.lite.account.AuthProvider
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.StarterActivationHandoff
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard
import com.product.ui.productScreenPadding

class AccountActivity : ComponentActivity() {
    private var completingLogin by mutableStateOf(false)
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        completingLogin = savedInstanceState?.getBoolean(EXTRA_RESUME_LOGIN)
            ?: intent.getBooleanExtra(EXTRA_RESUME_LOGIN, false)
        setContent {
            GovorunTheme {
                val state by GovorunAccountRepository.state.collectAsStateWithLifecycle()
                LaunchedEffect(state.signedIn, state.syncing, state.dataSyncMode) {
                    if (shouldFinishAccountLogin(completingLogin, state)) {
                        completingLogin = false
                        finishAccountLogin(this@AccountActivity)
                    }
                }
                val starterRequested = StarterActivationHandoff.pending(this)
                AccountScreen(
                    state = state,
                    starterRequested = starterRequested,
                    stylizationRequested = intent.getBooleanExtra(EXTRA_RETURN_TO_STYLIZATION, false),
                    finishingLogin = completingLogin,
                    onBack = ::finish,
                    onOpenSubscription = { startActivity(Intent(this, SubscriptionActivity::class.java)) },
                    onLogin = {
                        completingLogin = true
                        if (intent.getBooleanExtra(EXTRA_RETURN_TO_SUBSCRIPTION, false)) {
                            AccountLoginReturnHandoff.beginSubscription(this)
                        } else if (intent.getBooleanExtra(EXTRA_RETURN_TO_STYLIZATION, false)) {
                            AccountLoginReturnHandoff.beginStylization(this)
                        } else {
                            AccountLoginReturnHandoff.clear(this)
                        }
                        GovorunAccountRepository.startLogin(this, it)
                    },
                    onSignOut = GovorunAccountRepository::signOut,
                    onDeleteAccount = GovorunAccountRepository::deleteAccount,
                    onRetry = GovorunAccountRepository::refreshAccount,
                    onReplaceDevice = GovorunAccountRepository::replaceDeviceDuringLogin,
                    onRevokeDevice = GovorunAccountRepository::revokeDevice,
                    onMergeLocalData = GovorunAccountRepository::mergeLocalDataIntoAccount,
                    onKeepLocalData = GovorunAccountRepository::keepLocalDataOnly,
                    onRequestDataSync = GovorunAccountRepository::requestDataSyncDecision,
                )
            }
        }
    }

    override fun onResume() {
        super.onResume()
        GovorunAccountRepository.clearAbandonedLogin()
    }

    companion object {
        const val EXTRA_RETURN_TO_SUBSCRIPTION = "return_to_subscription"
        const val EXTRA_RESUME_LOGIN = "resume_explicit_login"
        const val EXTRA_RETURN_TO_STYLIZATION = "return_to_stylization"
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putBoolean(EXTRA_RESUME_LOGIN, completingLogin)
        super.onSaveInstanceState(outState)
    }
}

@Composable
internal fun AccountScreen(
    state: AccountState,
    starterRequested: Boolean,
    onBack: () -> Unit,
    onLogin: (AuthProvider) -> Unit,
    onSignOut: () -> Unit,
    onDeleteAccount: () -> Unit,
    onRetry: () -> Unit,
    onReplaceDevice: (String) -> Unit,
    onRevokeDevice: (String) -> Unit,
    onMergeLocalData: () -> Unit,
    onKeepLocalData: () -> Unit,
    onRequestDataSync: () -> Unit,
    onOpenSubscription: () -> Unit = {},
    stylizationRequested: Boolean = false,
    finishingLogin: Boolean = false,
) {
    var devicesOpen by rememberSaveable(state.accountId, state.signedIn) { mutableStateOf(false) }
    if (state.signedIn && devicesOpen) {
        BackHandler { devicesOpen = false }
        AccountDevicesScreen(state, onBack = { devicesOpen = false }, onRetry, onRevokeDevice)
        return
    }
    ProductScaffold(title = "Аккаунт", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
        ) {
            if (state.signedIn && finishingLogin) {
                item { Text("Завершаем вход…", style = MaterialTheme.typography.bodyMedium) }
            } else if (state.signedIn) {
                item {
                    AccountProfileCard(state, onSignOut, onDeleteAccount, onRequestDataSync)
                }
                if (state.secureStorageProblem != null || state.message != null) {
                    item { AccountStatusCard(state, onRetry) }
                }
                if (state.secureStorageProblem == null && state.dataSyncMode == AccountDataSyncMode.PENDING) {
                    item { BillingCard { DataSyncDecision(onMergeLocalData, onKeepLocalData) } }
                }
                item { AccountSubscriptionCard(state, onOpenSubscription) }
                item { AccountDevicesSummaryCard(state, onOpen = { devicesOpen = true }) }
            } else if (!state.deviceLimitReached) {
                item { AccountLoginCard(state, starterRequested, onLogin, onRetry, stylizationRequested) }
            }
            if (!state.signedIn && state.deviceLimitReached && state.devices.isNotEmpty()) {
                item { DevicesCard(state, onReplaceDevice, onRevokeDevice) }
            }
        }
    }
}

@Composable
private fun AccountLoginCard(
    state: AccountState,
    starterRequested: Boolean,
    onLogin: (AuthProvider) -> Unit,
    onRetry: () -> Unit,
    stylizationRequested: Boolean,
) {
    Card(
        modifier = Modifier.productFeedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(modifier = Modifier.padding(ProductTokens.Spacing.Section)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Surface(shape = CircleShape, color = MaterialTheme.colorScheme.primaryContainer) {
                    Icon(
                        PhosphorIcons.Regular.UserCircle,
                        contentDescription = null,
                        modifier = Modifier.padding(10.dp).size(24.dp),
                        tint = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
                Spacer(Modifier.width(ProductTokens.Spacing.Xl))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        if (state.secureStorageProblem != null && !state.signedIn) {
                            "Сохранённый аккаунт"
                        } else if (stylizationRequested) {
                            "Войти для улучшения текста"
                        } else if (starterRequested) {
                            "Войдите, чтобы попробовать"
                        } else {
                            "Войдите в Говорун"
                        },
                        style = MaterialTheme.typography.titleMedium,
                    )
                    if (!stylizationRequested) Text(
                        if (!state.signedIn && starterRequested) {
                            "Бесплатный тест сохранится в вашем аккаунте"
                        } else {
                            "Синхронизация настроек и статистики"
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (!stylizationRequested) {
                Spacer(Modifier.size(ProductTokens.Spacing.Xl))
                ProductDivider()
                Spacer(Modifier.size(ProductTokens.Spacing.Xl))
            }
            if (state.secureStorageProblem != null) {
                Text(state.secureStorageProblem.userMessage,
                    style = MaterialTheme.typography.bodyMedium)
                Button(onClick = onRetry, modifier = Modifier.fillMaxWidth()) { Text("Повторить") }
            } else {
                if (!stylizationRequested) Text(
                    if (starterRequested) {
                        "После входа новый аккаунт получит пробный доступ без карты и подписки. " +
                            "Если вы уже пользовались Говоруном, мы восстановим ваш текущий доступ."
                    } else {
                        "Войдите, чтобы синхронизировать настройки, словарь, стили и статистику."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                state.message?.let {
                    Text(
                        it,
                        modifier = Modifier.padding(top = ProductTokens.Spacing.Lg),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                AccountLoginLegalNotice(compact = stylizationRequested)
                Button(
                    enabled = state.available,
                    onClick = { onLogin(AuthProvider.YANDEX) },
                    modifier = Modifier.fillMaxWidth().padding(top = ProductTokens.Spacing.Xl),
                ) {
                    Text("Продолжить с Яндексом")
                }
                OutlinedButton(
                    enabled = state.available && BuildConfig.BUILD_TYPE != "billingTest",
                    onClick = { onLogin(AuthProvider.VK) },
                    modifier = Modifier.fillMaxWidth().padding(top = ProductTokens.Spacing.Lg),
                ) {
                    Text("Продолжить с VK ID")
                }
                if (BuildConfig.BUILD_TYPE == "billingTest") {
                    Text(
                        "Вход через VK ID пока недоступен на тестовом сервере.",
                        modifier = Modifier.padding(top = ProductTokens.Spacing.Lg),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun AccountLoginLegalNotice(compact: Boolean) {
    val linkStyle = TextLinkStyles(
        style = SpanStyle(
            color = MaterialTheme.colorScheme.primary,
            textDecoration = TextDecoration.Underline,
        ),
    )
    Text(
        text = buildAnnotatedString {
            append("Продолжая вход, вы принимаете ")
            withLink(LinkAnnotation.Url(LEGAL_URL, linkStyle)) {
                append("Условия и политика")
            }
            append(".")
        },
        modifier = Modifier.padding(top = ProductTokens.Spacing.Xl),
        style = if (compact) MaterialTheme.typography.bodyMedium else MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

private const val LEGAL_URL = "https://govorun.club/legal"
