package com.govorun.lite.account

internal data class StatsDay(
    val date: String,
    val words: Long,
    val speechSeconds: Long,
    val recognitions: Long,
)

internal data class StatsLifetimeTotal(
    val words: Long,
    val speechSeconds: Long,
    val recognitions: Long,
)
