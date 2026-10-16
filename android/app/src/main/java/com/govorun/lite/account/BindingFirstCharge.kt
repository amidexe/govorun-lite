package com.govorun.lite.account

internal fun bindingDelayCopy(seconds: Int): String {
    fun plural(value: Int, one: String, few: String, many: String): String = when {
        value % 100 in 11..14 -> many
        value % 10 == 1 -> one
        value % 10 in 2..4 -> few
        else -> many
    }
    val hours = seconds / 3600
    val minutes = seconds % 3600 / 60
    return buildList {
        if (hours > 0) add("$hours ${plural(hours, "час", "часа", "часов")}")
        if (minutes > 0) add("$minutes ${plural(minutes, "минуту", "минуты", "минут")}")
    }.joinToString(" ")
}
