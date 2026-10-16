package com.govorun.lite.account

internal fun tBankCheckoutErrorMessage(error: Throwable): String =
    checkoutConfirmationFailure(error)?.message ?: "Не удалось выполнить действие"
