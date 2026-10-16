package com.govorun.lite.ui

internal object AppBuildIdentity {
    fun versionLine(versionName: String, versionCode: Int): String =
        "Версия $versionName ($versionCode)"

    fun buildLine(buildId: String): String = "Сборка ${buildId.ifBlank { "—" }}"

    fun compactValue(versionName: String, versionCode: Int, buildId: String): String =
        "$versionName ($versionCode) · ${buildId.ifBlank { "—" }}"
}
