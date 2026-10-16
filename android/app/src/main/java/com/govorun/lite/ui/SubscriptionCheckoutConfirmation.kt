package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.style.TextAlign
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Sparkle
import com.govorun.lite.account.BillingPlan
import com.govorun.lite.account.TBankLegalDocuments
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productScreenPadding
import com.govorun.lite.account.bindingDelayCopy
import java.time.Instant
import java.time.ZoneId

internal data class SubscriptionCheckoutConfirmation(
    val plan: BillingPlan,
    val purchaseKind: String,
    val trialDays: Int,
    val freeAccessSource: String,
    val freeAccessEndsAt: String,
    val firstChargeAt: String,
    val receiptContactState: String,
    val receiptContactRequired: Boolean,
    val receiptEmailMasked: String,
    val receiptEmail: String = "",
    val amountDueNowKopeks: Int,
    val firstChargeAmountKopeks: Int,
    val renewalChargeAt: String,
    val renewalAmountKopeks: Int,
    val servicePeriodEndsAt: String,
    val subscriptionConsentToken: String,
    val legalDocuments: TBankLegalDocuments,
    val firstChargeDelaySeconds: Int? = null,
    val paymentMethodKind: String = "card",
    val availablePaymentMethods: List<String> = listOf("card"),
)

internal data class CheckoutConfirmationPresentation(
    val planTitle: String,
    val trialCopy: String,
    val amountDueNowCopy: String,
    val firstChargeCopy: String,
    val recurringPriceCopy: String,
    val providerHandoffCopy: String,
    val actionTitle: String,
    val consentCopy: String,
    val serverDateReady: Boolean,
)

internal data class CheckoutLegalLink(val title: String, val url: String)

internal fun checkoutLegalLinks(documents: TBankLegalDocuments): List<CheckoutLegalLink> = listOf(
    CheckoutLegalLink(
        title = "Условия и политика",
        url = documents.subscriptionTerms.url,
    ),
)

@Composable
internal fun SubscriptionCheckoutConfirmationScreen(
    state: SubscriptionCheckoutConfirmation,
    working: Boolean,
    paymentAvailable: Boolean = true,
    message: String?,
    onBack: () -> Unit,
    onContinue: (String?) -> Unit,
    onOpenLegalDocument: ((String) -> Result<Unit>)? = null,
    onPaymentMethod: (String) -> Unit = {},
) {
    val presentation = checkoutConfirmationPresentation(state)
    val uriHandler = LocalUriHandler.current
    var legalOpenFailed by rememberSaveable { mutableStateOf(false) }
    val receiptContactNeeded = state.receiptContactRequired
    var receiptEmail by rememberSaveable(state.plan.offerId, state.receiptEmail) {
        mutableStateOf(state.receiptEmail)
    }
    var consentAccepted by rememberSaveable(state.subscriptionConsentToken) {
        mutableStateOf(false)
    }
    val receiptContactReady = !receiptContactNeeded || plausibleReceiptEmail(receiptEmail)
    ProductScaffold(title = "Подключение подписки", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Section),
        ) {
            item { CheckoutSummaryCard(presentation) }
            if (state.purchaseKind == "subscription_start" && state.availablePaymentMethods.any { it != state.paymentMethodKind }) item {
                PaymentSourceChoice(state.paymentMethodKind, state.availablePaymentMethods, working, onPaymentMethod)
            }
            presentation.providerHandoffCopy.takeIf(String::isNotBlank)?.let { copy ->
                item {
                    Text(
                        copy,
                        modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Screen),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (receiptContactNeeded) item {
                ReceiptEmailSection(value = receiptEmail, onValueChange = { receiptEmail = it })
            } else item {
                Text(
                    "Чеки продолжат приходить на подтверждённый email текущей подписки.",
                    modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Screen),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            item {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Section),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.Top,
                    ) {
                        Checkbox(
                            checked = consentAccepted,
                            onCheckedChange = { consentAccepted = it },
                            enabled = !working,
                        )
                        Column(
                            modifier = Modifier.weight(1f).padding(top = ProductTokens.Spacing.Md),
                            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
                        ) {
                            Text(
                                presentation.firstChargeCopy,
                                style = MaterialTheme.typography.bodyMedium,
                            )
                            Text(
                                presentation.recurringPriceCopy,
                                style = MaterialTheme.typography.bodyMedium,
                            )
                            Text(
                                presentation.consentCopy,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    checkoutLegalLinks(state.legalDocuments).forEach { link ->
                        TextButton(onClick = {
                            val openFixed = onOpenLegalDocument
                            if (openFixed == null) uriHandler.openUri(link.url)
                            else legalOpenFailed = openFixed(link.url).isFailure
                        }) {
                            Text(link.title)
                        }
                    }
                    if (legalOpenFailed) {
                        Text("Не удалось открыть документ. Проверьте Firefox Nightly и тестовое подключение.")
                    }
                    Button(
                        onClick = {
                            onContinue(receiptEmail.trim().takeIf { receiptContactNeeded })
                        },
                        enabled = paymentAvailable && checkoutContinueEnabled(
                            serverDateReady = presentation.serverDateReady,
                            receiptContactReady = receiptContactReady,
                            consentAccepted = consentAccepted,
                            working = working,
                        ),
                        modifier = Modifier.fillMaxWidth().heightIn(
                            min = ProductTokens.Size.TouchTarget,
                        ),
                        shape = RoundedCornerShape(ProductTokens.Radius.Card),
                    ) {
                        Text(
                            if (working) "Подтверждаем…" else presentation.actionTitle,
                            style = MaterialTheme.typography.bodyMedium,
                        )
                    }
                    (if (!paymentAvailable) "Оплата временно недоступна. Условия можно проверить и вернуться позже."
                        else message)?.takeIf(String::isNotBlank)?.let {
                        Text(
                            it,
                            modifier = Modifier.padding(
                                horizontal = ProductTokens.Spacing.Screen,
                            ),
                            style = MaterialTheme.typography.bodySmall,
                            textAlign = TextAlign.Center,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }
        }
    }
}

internal fun checkoutContinueEnabled(
    serverDateReady: Boolean,
    receiptContactReady: Boolean,
    consentAccepted: Boolean,
    working: Boolean,
): Boolean = serverDateReady && receiptContactReady && consentAccepted && !working

@Composable
private fun ReceiptEmailSection(value: String, onValueChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = { if (it.length <= 64) onValueChange(it) },
        modifier = Modifier.fillMaxWidth(),
        label = { Text("Почта для чеков и уведомлений") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
        isError = value.isNotBlank() && !plausibleReceiptEmail(value),
        supportingText = { Text("Проверьте, что у вас есть доступ к этой почте.") },
    )
}

internal fun plausibleReceiptEmail(value: String): Boolean {
    val normalized = value.trim()
    if (normalized.isEmpty() || normalized.length > 64 || normalized.any(Char::isWhitespace)) {
        return false
    }
    val parts = normalized.split('@')
    return parts.size == 2 && parts[0].isNotEmpty() && parts[1].contains('.') &&
        !parts[1].startsWith('.') && !parts[1].endsWith('.')
}

@Composable
private fun CheckoutSummaryCard(presentation: CheckoutConfirmationPresentation) {
    BillingCard {
        Text(
            presentation.planTitle,
            style = MaterialTheme.typography.titleLarge,
        )
        Row(
            modifier = Modifier.padding(top = ProductTokens.Spacing.Lg),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            Icon(
                PhosphorIcons.Regular.Sparkle,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.SlotIcon),
                tint = MaterialTheme.colorScheme.primary,
            )
            Text(
                presentation.trialCopy,
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.primary,
            )
        }
        HorizontalDivider(modifier = Modifier.padding(vertical = ProductTokens.Spacing.Section))
        Text(
            presentation.amountDueNowCopy,
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}

internal fun checkoutConfirmationPresentation(
    state: SubscriptionCheckoutConfirmation,
    zoneId: ZoneId = ZoneId.systemDefault(),
): CheckoutConfirmationPresentation {
    val freeAccessEndDate = runCatching {
        formatRussianBillingDate(Instant.parse(state.freeAccessEndsAt).atZone(zoneId).toLocalDate())
    }.getOrNull()
    val firstChargeDate = runCatching {
        formatRussianBillingDate(Instant.parse(state.firstChargeAt).atZone(zoneId).toLocalDate())
    }.getOrNull()
    val firstChargeMoment = runCatching {
        formatRussianBillingMoment(Instant.parse(state.firstChargeAt).atZone(zoneId))
    }.getOrNull()
    val renewalChargeDate = runCatching {
        formatRussianBillingDate(Instant.parse(state.renewalChargeAt).atZone(zoneId).toLocalDate())
    }.getOrNull()
    val servicePeriodEndDate = runCatching {
        state.servicePeriodEndsAt.takeIf(String::isNotBlank)?.let(Instant::parse)
            ?.atZone(zoneId)?.toLocalDate()?.let(::formatRussianBillingDate)
    }.getOrNull()
    val dueNow = formatBillingKopeks(state.amountDueNowKopeks)
    val firstAmount = formatBillingKopeks(state.firstChargeAmountKopeks)
    val renewalAmount = formatBillingKopeks(state.renewalAmountKopeks)
    val planUpgrade = state.purchaseKind == "plan_upgrade"
    val trialUpgrade = state.purchaseKind == "trial_plan_upgrade"
    val scheduledChange = state.purchaseKind == "scheduled_offer_change"
    val bindingFirst = state.freeAccessSource == "binding_first"
    val shortCardTest = state.freeAccessSource == "card_trial" && state.trialDays == 0
    return CheckoutConfirmationPresentation(
        planTitle = listOfNotNull(state.plan.name, state.plan.clientMultiplierCopy).joinToString(" "),
        trialCopy = when {
            planUpgrade -> servicePeriodEndDate?.let { "Про до $it" } ?: "Переход на Про"
            trialUpgrade -> firstChargeDate?.let { "Про бесплатно до $it" }
                ?: "Про до конца пробного периода"
            scheduledChange -> servicePeriodEndDate?.let {
                "Текущий вариант действует до $it"
            } ?: "Смена при следующем продлении"
            bindingFirst -> "Привязка карты"
            shortCardTest -> "Проверка карты без списания"
            state.freeAccessSource == "promo" ->
                freeAccessEndDate?.let { "Промо действует до $it" } ?: "Промо уже действует"
            state.freeAccessSource == "current_period" ->
                freeAccessEndDate?.let { "Текущий тариф сохранится до $it" } ?: "Текущий доступ сохранится"
            state.freeAccessSource == "none" ->
                servicePeriodEndDate?.let { "Доступ до $it" } ?: "Без пробного периода"
            else -> "Первые ${state.trialDays} дней бесплатно"
        },
        amountDueNowCopy = if (planUpgrade) {
            "Доплата сейчас — $dueNow"
        } else {
            "Сейчас — $dueNow"
        },
        firstChargeCopy = when {
            planUpgrade -> renewalChargeDate?.let {
                "Следующее списание $it — $renewalAmount"
            } ?: "Дата следующего списания уточняется"
            scheduledChange -> firstChargeDate?.let { "Первое списание $it — $firstAmount" }
                ?: "Дата первого списания уточняется"
            state.amountDueNowKopeks > 0 -> renewalChargeDate?.let {
                "Следующее списание $it — $renewalAmount"
            } ?: "Дата следующего списания уточняется"
            bindingFirst -> if (state.firstChargeDelaySeconds != null)
                "Первое списание $firstAmount через ${bindingDelayCopy(state.firstChargeDelaySeconds!!)} после привязки карты"
                else "Проверяем условия первого списания"
            shortCardTest -> firstChargeMoment?.let { "Первое списание $it — $firstAmount" }
                ?: "Первое списание — время уточняется"
            else -> firstChargeDate?.let { "Первое списание $it — $firstAmount" }
                ?: "Первое списание — дата уточняется"
        },
        recurringPriceCopy = "Далее $renewalAmount / ${state.plan.billingPeriodLabel}",
        providerHandoffCopy = when {
            state.paymentMethodKind == "sbp" -> "Подтвердите привязку счёта в приложении вашего банка. Данные карты вводить не нужно."
            state.amountDueNowKopeks == 0 -> ""
            !planUpgrade -> "Сначала привяжите карту за 0 ₽ в банке. Затем спишем $dueNow за подписку."
            else ->
                "Оплата откроется на защищённой странице Т‑Банка. " +
                    "Результат Говорун проверит по данным банка."
        },
        actionTitle = when {
            planUpgrade -> "Доплатить $dueNow"
            trialUpgrade -> "Перейти на Про"
            scheduledChange -> "Сменить тариф"
            state.amountDueNowKopeks > 0 -> "Оплатить $dueNow"
            state.paymentMethodKind == "sbp" -> "Подключить СБП"
            else -> "Привязать карту"
        },
        consentCopy = "Подтверждаю показанные условия подписки и автопродление. " +
            "Автопродление можно отключить в любой момент. " +
            "Разрешаю передать подтверждённый email ООО «КЛАУДКАССИР» и ОФД " +
            "для отправки чеков, а Говоруну — присылать обязательные уведомления " +
            "о состоянии оплаты.",
        serverDateReady = if (bindingFirst) state.firstChargeDelaySeconds != null
            else firstChargeDate != null && renewalChargeDate != null &&
                (!(planUpgrade || scheduledChange) || servicePeriodEndDate != null),
    )
}


private fun formatRussianBillingDate(date: java.time.LocalDate): String {
    val months = listOf(
        "января", "февраля", "марта", "апреля", "мая", "июня",
        "июля", "августа", "сентября", "октября", "ноября", "декабря",
    )
    return "${date.dayOfMonth} ${months[date.monthValue - 1]} ${date.year} года"
}

private fun formatRussianBillingMoment(value: java.time.ZonedDateTime): String =
    "${formatRussianBillingDate(value.toLocalDate())}, " +
        "%02d:%02d".format(value.hour, value.minute)
