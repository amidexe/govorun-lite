package com.govorun.lite.ui.home

import com.govorun.lite.account.AccountState
import com.govorun.lite.account.StyleAccess
import com.govorun.lite.account.currentOnlineTermsActionRequired
import com.govorun.lite.account.styleSetupNeedsSubscription
import com.govorun.lite.account.connectionMessage
import com.govorun.lite.account.connectionHint
import com.govorun.lite.network.NetworkRefusal
import java.time.Duration
import java.time.Instant

internal data class HomeAccessPresentation(
    val active: Boolean,
    val label: String,
    val actionTitle: String,
    val actionHint: String,
)

internal fun shouldShowOnlineAccessOnHome(
    coreSetupReady: Boolean,
    accessAvailable: Boolean,
): Boolean = coreSetupReady && accessAvailable

internal fun homeAccessPresentation(
    account: AccountState,
    now: Instant = Instant.now(),
): HomeAccessPresentation = when {
    !account.signedIn -> HomeAccessPresentation(
        active = false,
        label = "Онлайн-нейросеть учитывает контекст, " +
            "убирает повторы и превращает устную речь в связный текст. " +
            "Меньше ручных правок после диктовки.",
        actionTitle = "Попробовать улучшения",
        actionHint = "Диктовка работает и без входа",
    )
    account.connectionMessage != null -> HomeAccessPresentation(
        active = false,
        label = "${account.connectionMessage}. Диктовка работает офлайн.",
        actionTitle = when (account.networkRefusal.takeUnless { account.networkAvailable }) {
            NetworkRefusal.OFFLINE -> "Включить сеть"
            NetworkRefusal.ROAMING -> "Настройки"
            null -> "Повторить"
        },
        actionHint = account.connectionHint,
    )
    styleSetupNeedsSubscription(account) -> HomeAccessPresentation(
        active = false,
        label = if (account.styleAccess.type == "starter") "Бесплатный лимит использован" else "Для улучшения текста нужен доступ",
        actionTitle = "Посмотреть тарифы",
        actionHint = "Выберите подходящий тариф",
    )
    account.hasEntitlement -> HomeAccessPresentation(
        active = true,
        label = activeAccessLabel(account.styleAccess, now),
        actionTitle = "Выбрать стиль",
        actionHint = "Доступ уже подключён",
    )
    account.starterGrant.eligible -> HomeAccessPresentation(
        active = false,
        label = "Улучшайте текст после диктовки",
        actionTitle = "Включить",
        actionHint = "Без карты и подписки",
    )
    account.starterLocallyExhausted -> HomeAccessPresentation(
        active = false,
        label = "Бесплатный лимит использован",
        actionTitle = "Посмотреть тарифы",
        actionHint = "Доступны Базовая и Про",
    )
    else -> HomeAccessPresentation(
        active = false,
        label = "Улучшение текста",
        actionTitle = "Посмотреть тарифы",
        actionHint = "Выберите вариант доступа",
    )
}

internal fun requiresOnlineTermsAction(
    selfHosted: Boolean,
    officialStyleSelected: Boolean,
    localCurrentTermsAccepted: Boolean,
    account: AccountState,
): Boolean = !selfHosted && officialStyleSelected && account.hasEntitlement && !styleSetupNeedsSubscription(account) && (
    currentOnlineTermsActionRequired(localCurrentTermsAccepted, account)
    )

internal fun activeAccessLabel(access: StyleAccess, now: Instant): String {
    if (access.type == "promo_program") return "Промо · доступ активен"
    if (access.type == "starter") return "Бесплатный лимит подключён"
    if (access.type == "paid" || access.type == "plan_grant") {
        return if (access.planCode == "pro") "Про · доступ активен"
        else "Базовая · доступ активен"
    }
    val remainingDays = remainingAccessDays(access.endsAt, now)
    return if (remainingDays == null || remainingDays <= 0) {
        "Бонусный лимит подключён"
    } else {
        bonusAccessLabel(remainingDays)
    }
}

internal fun bonusAccessLabel(remainingDays: Int): String {
    val ending = when {
        remainingDays % 100 in 11..14 -> "дней"
        remainingDays % 10 == 1 -> "день"
        remainingDays % 10 in 2..4 -> "дня"
        else -> "дней"
    }
    return "Бонусный лимит · ещё $remainingDays $ending"
}

internal fun remainingAccessDays(endsAt: String, now: Instant): Int? {
    val end = runCatching { Instant.parse(endsAt) }.getOrNull() ?: return null
    val seconds = Duration.between(now, end).seconds
    if (seconds <= 0) return 0
    return ((seconds + SECONDS_PER_DAY - 1) / SECONDS_PER_DAY).toInt()
}

private const val SECONDS_PER_DAY = 24L * 60L * 60L
