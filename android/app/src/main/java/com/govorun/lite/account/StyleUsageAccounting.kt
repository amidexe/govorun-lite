package com.govorun.lite.account

typealias StyleUsageBalance = com.govorun.core.generated.ConsumerStyleUsageBalance
internal val AccountState.stylePauseMessage: String?
    get() {
        if (secureStorageProblem != null) return "Не удалось прочитать сохранённый вход"
        if (!hasEntitlement) return null
        return when (styleWindowPresentation().pauseReason) {
            "none" -> null
            "refreshing" -> "Обновляем остаток"
            "periodLimit" -> if (styleAccess.balanceKind == "single_grant") "Бесплатный лимит закончился" else "Запас за период закончился"
            "dailyLimit" -> "Лимит на сегодня закончился"
            else -> "Стилизация временно недоступна"
        }
    }
