package com.govorun.lite.ui

import com.govorun.lite.account.active
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.govorun.lite.account.AccountState
import com.govorun.lite.account.BillingTrialOffer
import com.govorun.lite.account.PromoOfferPreview
import com.govorun.lite.account.normalizePromoCode
import com.govorun.lite.account.StarterGrant
import com.govorun.lite.account.StarterGrantOffer
import com.govorun.lite.account.starterStyleLimitExhausted
import com.product.ui.ProductTokens

internal class PromoCodeEntryState(
    val visible: Boolean = false,
    val code: String = "",
) {
    fun open(): PromoCodeEntryState = PromoCodeEntryState(visible = true)

    fun change(value: String): PromoCodeEntryState =
        if (visible) PromoCodeEntryState(visible = true, code = value) else this

    fun close(): PromoCodeEntryState = PromoCodeEntryState()
}

internal fun promoCodeReadyForSubmit(
    code: String,
    working: Boolean,
): Boolean = !working && runCatching { normalizePromoCode(code) }.isSuccess

@Composable
internal fun PromoOfferConfirmationDialog(
    preview: com.govorun.lite.account.PromotionPreview,
    working: Boolean,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(preview.title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
                Text(preview.body)
                preview.bonusDays?.let { days ->
                    Text("Дополнительный запас на $days дней")
                }
                if (preview.checkoutRequired) {
                    Text(
                        "Подтверждение сохраняет предложение. Подписка и доступ появятся только " +
                            "после отдельной подтверждённой оплаты.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(enabled = !working, onClick = onConfirm) { Text("Подтвердить") }
        },
        dismissButton = {
            TextButton(enabled = !working, onClick = onDismiss) { Text("Отмена") }
        },
    )
}

@Composable
internal fun PromoCodeAction(
    signedIn: Boolean,
    working: Boolean,
    onEnterCode: () -> Unit,
    onSignIn: () -> Unit,
) {
    TextButton(
        onClick = if (signedIn) onEnterCode else onSignIn,
        enabled = !working,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(
            when {
                working -> "Активируем…"
                else -> "Ввести промокод"
            },
        )
    }
}

@Composable
internal fun PromoCodeDialog(
    code: String,
    working: Boolean,
    onCodeChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Промокод") },
        text = {
            OutlinedTextField(
                value = code,
                onValueChange = onCodeChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text("Введите код") },
            )
        },
        confirmButton = {
            TextButton(
                enabled = promoCodeReadyForSubmit(code, working),
                onClick = onSubmit,
            ) {
                Text("Активировать")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Отмена") }
        },
    )
}

internal enum class SubscriptionAccessStage {
    STARTER_OFFER,
    STARTER_ACTIVE,
    BONUS_ACTIVE,
    PAID_ACTIVE,
    PLANS,
}

internal data class TariffDisclosure(
    val discoveryActionVisible: Boolean,
    val cardsVisible: Boolean,
    val checkoutVisible: Boolean,
)

internal data class SubscriptionIntro(
    val title: String,
    val body: String,
)

internal fun subscriptionIntro(
    stage: SubscriptionAccessStage,
    starterExhausted: Boolean = false,
    trialDays: Int? = null,
): SubscriptionIntro = when (stage) {
    SubscriptionAccessStage.STARTER_OFFER -> SubscriptionIntro(
        title = "Попробуйте улучшение текста",
        body = "Выберите стиль и получите результат, который вам больше нравится.",
    )
    SubscriptionAccessStage.STARTER_ACTIVE -> SubscriptionIntro(
        title = "Бесплатный лимит подключён",
        body = "Выберите стиль и получите результат, который вам больше нравится.",
    )
    SubscriptionAccessStage.BONUS_ACTIVE -> SubscriptionIntro(
        title = "Улучшение текста подключено",
        body = "Можно пользоваться прямо сейчас.",
    )
    SubscriptionAccessStage.PAID_ACTIVE -> SubscriptionIntro(
        title = "Улучшение текста подключено",
        body = "",
    )
    SubscriptionAccessStage.PLANS -> SubscriptionIntro(
        title = if (starterExhausted && trialDays == null) "Бесплатный лимит закончился" else "Продолжите улучшать текст",
        body = if (trialDays != null) "Выберите тариф, чтобы продолжить улучшать текст."
            else if (starterExhausted) "Выберите тариф, чтобы продолжить улучшать текст. Офлайн-диктовка остаётся бесплатной."
            else "Выберите тариф. Распознавание речи работает офлайн, а подписка нужна только для улучшения текста.",
    )
}

@Suppress("UNUSED_PARAMETER")
internal fun tariffDisclosure(
    stage: SubscriptionAccessStage,
    expandedBeforeExhaustion: Boolean,
): TariffDisclosure = TariffDisclosure(
    discoveryActionVisible = false, cardsVisible = true, checkoutVisible = true,
)

internal fun subscriptionAccessStage(
    account: AccountState,
    catalogStarter: StarterGrantOffer,
    billingStarter: StarterGrant,
): SubscriptionAccessStage {
    if (starterStyleLimitExhausted(account)) return SubscriptionAccessStage.PLANS
    if (account.styleAccess.active && account.signedIn) {
        return when (account.styleAccess.type) {
            "starter" -> SubscriptionAccessStage.STARTER_ACTIVE
            "paid", "trial", "plan_grant" -> SubscriptionAccessStage.PAID_ACTIVE
            else -> SubscriptionAccessStage.BONUS_ACTIVE
        }
    }
    val starter = if (account.signedIn) account.starterGrant else billingStarter
    val starterAvailable = if (account.signedIn) starter.eligible else catalogStarter.clientOfferVisible
    return if (starterAvailable) {
        SubscriptionAccessStage.STARTER_OFFER
    } else {
        SubscriptionAccessStage.PLANS
    }
}

internal fun shouldOfferProUpgrade(
    accessType: String,
    activePlanCode: String,
    candidatePlanCode: String,
    periodRemainingPercent: Int?,
): Boolean = accessType == "paid" && activePlanCode == "base" &&
    candidatePlanCode == "pro" && periodRemainingPercent != null &&
    periodRemainingPercent <= 20
