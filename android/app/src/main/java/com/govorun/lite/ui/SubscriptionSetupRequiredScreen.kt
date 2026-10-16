package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard
import com.product.ui.productScreenPadding

@Composable
internal fun SubscriptionSetupRequiredScreen(
    onBack: () -> Unit,
    onCompleteSetup: () -> Unit,
) {
    SubscriptionEntryStepScreen(
        title = "Сначала завершите настройку Говоруна",
        body = "Проверьте микрофон и специальные возможности. После этого здесь появятся стили и варианты доступа.",
        actionTitle = "Завершить настройку", onBack = onBack, onAction = onCompleteSetup,
    )
}

@Composable
internal fun SubscriptionSignInRequiredScreen(onBack: () -> Unit, onSignIn: () -> Unit) {
    SubscriptionEntryStepScreen(
        title = "Войдите в аккаунт",
        body = "После входа появятся ваш доступ и тарифы. Обычная диктовка работает без аккаунта.",
        actionTitle = "Войти", onBack = onBack, onAction = onSignIn,
    )
}

internal enum class SubscriptionEntryStep { SETUP, SIGN_IN, ACCESS }

internal fun subscriptionEntryStep(
    coreReady: Boolean,
    signedIn: Boolean,
    demo: Boolean = false,
) = when {
    !coreReady -> SubscriptionEntryStep.SETUP
    !signedIn && !demo -> SubscriptionEntryStep.SIGN_IN
    else -> SubscriptionEntryStep.ACCESS
}

@Composable
private fun SubscriptionEntryStepScreen(
    title: String, body: String, actionTitle: String,
    onBack: () -> Unit, onAction: () -> Unit,
) {
    ProductScaffold(title = "Улучшение текста", onBack = onBack) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding)
                .padding(productScreenPadding()),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
        ) {
            Column(
                modifier = Modifier.fillMaxWidth().productFeedCard()
                    .padding(ProductTokens.Spacing.Xl),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                Text(
                    title,
                    style = MaterialTheme.typography.headlineSmall,
                    textAlign = TextAlign.Center,
                )
                Text(
                    body,
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Button(onClick = onAction, modifier = Modifier.fillMaxWidth()) {
                Text(actionTitle)
            }
        }
    }
}

@Composable
internal fun SubscriptionStorageUnavailableScreen(onBack: () -> Unit, onOpenAccount: () -> Unit) {
    SubscriptionEntryStepScreen(
        title = "Не удалось прочитать сохранённый вход",
        body = "Диктовка работает без стилизации. Откройте аккаунт и повторите попытку.",
        actionTitle = "Открыть аккаунт", onBack = onBack, onAction = onOpenAccount,
    )
}
