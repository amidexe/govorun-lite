package com.govorun.lite.account

typealias StyleUsageBalance = com.govorun.core.generated.ConsumerStyleUsageBalance
internal val AccountState.stylePauseMessage: String?
    get() {
        if (secureStorageProblem != null) return "Не удалось прочитать сохранённый вход"
        if (!hasEntitlement) return null
        return when (styleWindowPresentation().pauseReason) {
            "none" -> null
            "refreshing" -> "Обновляем остаток"
            // Calm words: dictation keeps working, only the online improvement waits.
            "periodLimit" -> if (styleAccess.balanceKind == "single_grant") "Бесплатный лимит закончился"
                else "Улучшение за 30 дней закончилось\nДиктовка работает как обычно"
            "dailyLimit" -> styleWindowPresentation().quota
                ?.todayResetTime(java.time.ZoneId.systemDefault(), java.util.Locale.forLanguageTag("ru"))
                ?.let { "Улучшение на сегодня закончилось\nДиктовка работает как обычно, улучшение вернётся в $it" }
                ?: "Улучшение на сегодня закончилось\nДиктовка работает как обычно"
            else -> "Стилизация временно недоступна"
        }
    }
