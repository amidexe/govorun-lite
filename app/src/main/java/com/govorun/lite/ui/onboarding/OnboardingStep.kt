package com.govorun.lite.ui.onboarding

/**
 * Ordered list of onboarding steps. The ordinal is the page index in the ViewPager2.
 * Adding a new step anywhere in the middle stays safe — persistence stores the enum
 * name, not the ordinal.
 */
enum class OnboardingStep {
    WELCOME,
    TRY_IT,
    ACCESSIBILITY,
    BATTERY
}
