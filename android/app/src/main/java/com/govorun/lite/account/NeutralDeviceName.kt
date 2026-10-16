package com.govorun.lite.account

import android.os.Build
import java.util.Locale

internal object NeutralDeviceName {
    fun current(): String = from(Build.MANUFACTURER, Build.MODEL)

    fun from(manufacturer: String?, model: String?): String {
        val cleanManufacturer = clean(manufacturer)
            .takeUnless { it.equals("unknown", ignoreCase = true) }
        val cleanModel = clean(model)
            .takeUnless { it.equals("unknown", ignoreCase = true) }
        val value = when {
            cleanModel == null -> cleanManufacturer
            cleanManufacturer == null -> cleanModel
            cleanModel.contains(cleanManufacturer, ignoreCase = true) -> cleanModel
            else -> "${brandCase(cleanManufacturer)} $cleanModel"
        } ?: "Android"
        return value.take(MAXIMUM_LENGTH).trim().ifBlank { "Android" }
    }

    private fun clean(value: String?): String? = value
        ?.filterNot(Char::isISOControl)
        ?.trim()
        ?.split(Regex("\\s+"))
        ?.joinToString(" ")
        ?.takeIf(String::isNotBlank)

    private fun brandCase(value: String): String = value.replaceFirstChar {
        if (it.isLowerCase()) it.titlecase(Locale.ROOT) else it.toString()
    }

    private const val MAXIMUM_LENGTH = 64
}
