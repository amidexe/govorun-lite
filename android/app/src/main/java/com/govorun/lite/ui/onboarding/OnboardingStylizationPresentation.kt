package com.govorun.lite.ui.onboarding

import com.govorun.lite.account.supportsTextCreditProtocol
import com.govorun.lite.account.AccountState

internal data class OnboardingStylizationPresentation(
    val canEnable: Boolean,
    val showAccessAction: Boolean,
    val status: String,
    val accessActionTitle: String,
)

internal fun onboardingStylizationPresentation(
    stylesConfigured: Boolean,
    account: AccountState,
): OnboardingStylizationPresentation {
    val canEnable = stylesConfigured && account.hasEntitlement
    val status = when {
        !stylesConfigured -> "Улучшение текста недоступно"
        !account.signedIn -> "Войдите, чтобы подключить стили"
        account.hasEntitlement && !account.styleAccess.supportsTextCreditProtocol ->
            "Улучшение текста временно недоступно"
        account.hasOfficialAccess && !account.onlineService.available ->
            "Улучшение текста временно недоступно"
        account.hasOfficialAccess && account.styleAccess.type == "promo_program" ->
            "Промо · доступ активен"
        account.hasOfficialAccess && account.styleAccess.type == "starter" ->
            "Бесплатный лимит подключён"
        account.hasOfficialAccess && account.styleAccess.type == "paid" -> when (
            account.styleAccess.planCode
        ) {
            "pro" -> "Про · доступ активен"
            else -> "Базовая · доступ активен"
        }
        account.hasOfficialAccess && account.styleAccess.type == "trial" ->
            "Пробный период подключён"
        account.hasOfficialAccess -> "Бонусный лимит подключён"
        account.hasEntitlement -> "Примите условия интернет-обработки"
        account.starterGrant.eligible -> "Попробуйте улучшение текста бесплатно"
        else -> "Для стилей нужен активный доступ"
    }
    val action = when {
        !account.signedIn -> "Войти и открыть доступ"
        account.hasEntitlement -> "Выбрать стиль"
        account.starterGrant.eligible -> "Попробовать бесплатно"
        else -> "Посмотреть варианты"
    }
    return OnboardingStylizationPresentation(
        canEnable = canEnable,
        showAccessAction = stylesConfigured && !account.hasEntitlement,
        status = status,
        accessActionTitle = action,
    )
}
