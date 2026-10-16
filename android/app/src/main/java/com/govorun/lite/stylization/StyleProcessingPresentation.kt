package com.govorun.lite.stylization

import com.govorun.lite.account.AccountState
import com.govorun.lite.account.StyleProcessingReadiness
import com.govorun.lite.account.styleWindowPresentation

/** Native copy for the common readiness decision; the saved selection stays independent. */
internal data class StyleProcessingPresentation(val ready: Boolean, val message: String?)

internal fun AccountState.styleProcessingPresentation(): StyleProcessingPresentation =
    when (styleWindowPresentation().processingReadiness) {
        StyleProcessingReadiness.READY -> StyleProcessingPresentation(true, null)
        StyleProcessingReadiness.ON_DEMAND -> StyleProcessingPresentation(false, null)
        StyleProcessingReadiness.PREPARING ->
            StyleProcessingPresentation(false, "Подготавливаем стилизацию…")
        StyleProcessingReadiness.NEEDS_PREPARATION ->
            StyleProcessingPresentation(false, "Стилизация пока не готова")
        StyleProcessingReadiness.UNAVAILABLE ->
            StyleProcessingPresentation(false, "Улучшение текста временно недоступно")
    }
