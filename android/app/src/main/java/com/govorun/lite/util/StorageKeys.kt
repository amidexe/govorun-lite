package com.govorun.lite.util

/** Current preference names; preserve them when changing UI or adapters. */
internal object StorageKeys {
    const val PREFS_FILE = "govorun_lite_prefs"

    object Key {
        const val HAPTICS_ENABLED = "haptics_enabled"
        const val HIDE_IN_SEARCH = "hide_in_search"
        const val BUBBLE_ALPHA = "bubble_alpha"
        const val BUBBLE_RECORDING_ALPHA = "bubble_recording_alpha"
        const val BUBBLE_COLOR = "bubble_color"
        const val BUBBLE_ALPHA_CHOSEN_FOR_COLOR = "bubble_alpha_chosen_for_color"
        const val BUBBLE_SIZE = "bubble_size"
        const val BUBBLE_Y = "bubble_y"
        const val BUBBLE_Y_PORTRAIT = "bubble_y_portrait"
        const val BUBBLE_Y_LANDSCAPE = "bubble_y_landscape"
        const val BUBBLE_SIDE = "bubble_side"
        const val BUBBLE_EDGE_MARGIN = "bubble_edge_margin"
        const val BUBBLE_POSITION_LOCKED = "bubble_position_locked"
        const val DICTIONARY = "dictionary_text"
        const val DICTIONARY_ENABLED = "dictionary_enabled"
        const val PAUSE_LENGTH = "pause_length"
        const val KEEP_SCREEN_ON = "keep_screen_on"
        const val REMEMBER_LAST_DICTATION = "remember_last_dictation"
        const val AUTO_STOP_MINUTES = "auto_stop_minutes"
        const val STYLIZATION_ENABLED = "stylization_enabled"
        const val STYLIZATION_STYLE = "stylization_style"
        const val STYLIZATION_CONSENT_VERSION = "stylization_consent_version"
        const val STYLIZATION_CONSENT_AT = "stylization_consent_at"
        const val APP_FILTER_MODE = "app_filter_mode"
        const val APP_FILTER_PACKAGES = "app_filter_packages"
        const val ONBOARDING_STEP = "onboarding_step"
        const val ONBOARDING_DONE = "onboarding_done"
        const val STATS_WORDS = "stats_words_total"
        const val STATS_SECONDS = "stats_seconds_total"
        const val STATS_WORDS_TODAY = "stats_words_today"
        const val STATS_TODAY_DATE = "stats_today_date"
    }
}
