package com.govorun.lite.ui

import com.govorun.lite.account.BillingSubscription

import java.time.Instant
import java.time.temporal.ChronoUnit

/** Synthetic screen data only; this never publishes account authority or sends a payment. */
internal object BillingDemoScenario {
    fun purchaseMessage(current: BillingSubscription, planCode: String): String = when {
        current.planCode == "base" && planCode == "pro" ->
            "Тестовое повышение выполнено. Доступен полный дневной лимит Про"
        current.planCode == "pro" && planCode == "base" ->
            "Тариф сменится на Базовую после текущего периода"
        else -> "Тестовая подписка подключена. Деньги не списывались"
    }

    fun purchase(
        current: BillingSubscription,
        planCode: String,
        now: Instant = Instant.now(),
    ): BillingSubscription {
        require(planCode == "base" || planCode == "pro")
        if (current.planCode == "pro" && planCode == "base" && current.state == "active") {
            return current.copy(
                pendingPlanCode = "base",
                changeEffectiveAt = current.currentPeriodEndsAt,
                renewsAutomatically = false,
            )
        }
        return BillingSubscription(
            state = "active",
            planCode = planCode,
            currentPeriodEndsAt = now.plus(30, ChronoUnit.DAYS).toString(),
            renewsAutomatically = true,
            serviceAccessState = "active",
        )
    }

    fun cancel(current: BillingSubscription): BillingSubscription =
        current.copy(renewsAutomatically = false, pendingPlanCode = "")
}
