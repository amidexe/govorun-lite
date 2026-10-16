package com.govorun.lite.ui.settings

import com.govorun.lite.account.AccountState
import com.govorun.lite.account.activeStyleLimitPresentation
import com.govorun.lite.account.starterStyleLimitPresentation

internal fun settingsAccessSummary(account: AccountState): String {
    if (!account.available) return "Улучшение текста пока недоступно"
    if (!account.signedIn) return "Войдите, чтобы увидеть тариф и остаток"

    val plan = when {
        account.starterLocallyExhausted -> "Пробный доступ использован"
        account.styleAccess.type == "promo_program" -> "Промо"
        account.styleAccess.type == "starter" -> "Бесплатный лимит подключён"
        account.styleAccess.balanceKind == "single_bonus" -> "Бонусный лимит подключён"
        account.styleAccess.type == "trial" -> "Пробный период подключён"
        account.styleAccess.planCode == "base" -> "Базовая"
        account.styleAccess.planCode == "pro" -> "Про"
        else -> "Подписка"
    }
    if (account.starterLocallyExhausted) return plan
    if (account.styleAccess.type == "starter") {
        return starterStyleLimitPresentation(account)?.let {
            "Бесплатный лимит · ${it.status.lowercase()}"
        } ?: plan
    }
    if (
        account.styleAccess.balanceKind == "single_bonus" ||
        account.styleAccess.type == "trial"
    ) {
        return plan
    }
    if (activeStyleLimitPresentation(account) != null) {
        if (account.styleAccess.type == "promo_program") return "Промо · доступ активен"
        return when (account.styleAccess.planCode) {
            "base" -> "Базовая · доступ активен"
            "pro" -> "Про · доступ активен"
            else -> "$plan подключена"
        }
    }
    if (account.hasEntitlement) return "$plan · остаток обновляется"
    if (account.starterGrant.eligible) return "Попробуйте бесплатно"
    return "Базовая или Про"
}
