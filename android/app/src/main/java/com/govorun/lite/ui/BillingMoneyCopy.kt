package com.govorun.lite.ui

internal fun formatBillingKopeks(value: Int): String {
    require(value >= 0)
    return if (value % 100 == 0) {
        "${value / 100} ₽"
    } else {
        "%d,%02d ₽".format(value / 100, value % 100)
    }
}
