package com.govorun.lite.ui

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.database.ContentObserver
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.provider.Settings
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButton
import com.google.android.material.card.MaterialCardView
import com.google.android.material.color.DynamicColors
import com.google.android.material.textview.MaterialTextView

import com.govorun.lite.R
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.util.AccessibilityHelper

/**
 * Post-onboarding home screen.
 *
 *  - AppBar with overflow menu: Настройки / Pro-версия / О программе
 *  - Headline / subline adapt: ready / needs setup / just finished
 *  - Per-problem cards (mic / service / battery) show only when their specific
 *    check fails, each with a dedicated deep-link button
 *  - Last-try memento card surfaces the text from the onboarding demo
 *  - Shortcut advisory card warns about the small accessibility button
 *  - Stats card always visible (shows empty-state hint before first use)
 *  - Promo card links to FuturePlansActivity
 */
class MainActivity : AppCompatActivity() {

    private lateinit var headline: MaterialTextView
    private lateinit var subline: MaterialTextView

    private lateinit var cardMicMissing: MaterialCardView
    private lateinit var cardMicButton: MaterialButton
    private lateinit var cardServiceMissing: MaterialCardView
    private lateinit var cardServiceButton: MaterialButton
    private lateinit var cardBatteryMissing: MaterialCardView
    private lateinit var cardBatteryButton: MaterialButton

    private lateinit var shortcutCard: View
    private lateinit var shortcutCardButton: MaterialButton
    private lateinit var statsCard: View
    private lateinit var statsRow: View
    private lateinit var statsWordsNumber: MaterialTextView
    private lateinit var statsMinutesNumber: MaterialTextView
    private lateinit var statsToday: MaterialTextView
    private lateinit var statsEmpty: MaterialTextView
    private lateinit var statsActionRow: View
    private lateinit var statsShareButton: MaterialButton
    private lateinit var promoCard: View
    private lateinit var toolbar: MaterialToolbar
    private var showJustFinished: Boolean = false

    // Observes accessibility-related Secure settings so the per-problem cards
    // refresh the instant the user toggles the service OR taps the little
    // shortcut button on top of our bubble — no need to leave MainActivity.
    private val accessibilityObserver = object : ContentObserver(Handler(Looper.getMainLooper())) {
        override fun onChange(selfChange: Boolean) {
            refreshStatuses()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        DynamicColors.applyToActivityIfAvailable(this)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        toolbar = findViewById(R.id.topAppBar)
        toolbar.setOnMenuItemClickListener { item ->
            when (item.itemId) {
                R.id.action_settings -> {
                    startActivity(Intent(this, SettingsActivity::class.java))
                    true
                }
                R.id.action_pro -> {
                    startActivity(Intent(this, FuturePlansActivity::class.java))
                    true
                }
                R.id.action_about -> {
                    startActivity(Intent(this, AboutActivity::class.java))
                    true
                }
                else -> false
            }
        }

        val scroll = findViewById<View>(R.id.scroll)
        ViewCompat.setOnApplyWindowInsetsListener(scroll) { v, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.navigationBars() or WindowInsetsCompat.Type.displayCutout()
            )
            v.updatePadding(bottom = bars.bottom)
            insets
        }

        headline = findViewById(R.id.mainHeadline)
        subline = findViewById(R.id.mainSubline)

        cardMicMissing = findViewById(R.id.cardMicMissing)
        cardMicButton = findViewById(R.id.cardMicButton)
        cardMicButton.setOnClickListener { openAppDetails() }

        cardServiceMissing = findViewById(R.id.cardServiceMissing)
        cardServiceButton = findViewById(R.id.cardServiceButton)
        cardServiceButton.setOnClickListener {
            AccessibilityHelper.openAccessibilitySettings(this)
        }

        cardBatteryMissing = findViewById(R.id.cardBatteryMissing)
        cardBatteryButton = findViewById(R.id.cardBatteryButton)
        cardBatteryButton.setOnClickListener { requestIgnoreBatteryOptimizations() }

        shortcutCard = findViewById(R.id.shortcutCard)
        shortcutCardButton = findViewById(R.id.shortcutCardButton)
        shortcutCardButton.setOnClickListener {
            AccessibilityHelper.openAccessibilitySettings(this)
        }
        statsCard = findViewById(R.id.statsCard)
        statsRow = findViewById(R.id.statsRow)
        statsWordsNumber = findViewById(R.id.statsWordsNumber)
        statsMinutesNumber = findViewById(R.id.statsMinutesNumber)
        statsToday = findViewById(R.id.statsToday)
        statsEmpty = findViewById(R.id.statsEmpty)
        statsActionRow = findViewById(R.id.statsActionRow)
        statsShareButton = findViewById(R.id.statsShareButton)
        statsShareButton.setOnClickListener { shareAppLink() }

        promoCard = findViewById(R.id.promoCard)
        promoCard.setOnClickListener {
            startActivity(Intent(this, FuturePlansActivity::class.java))
        }

        showJustFinished = intent.getBooleanExtra(EXTRA_JUST_FINISHED, false)
        if (showJustFinished) {
            // Consume the flag so orientation changes / re-launches show the
            // standard headline.
            intent.removeExtra(EXTRA_JUST_FINISHED)
        }
    }

    override fun onResume() {
        super.onResume()
        refreshStatuses()
        refreshStats()
        val cr = contentResolver
        cr.registerContentObserver(
            Settings.Secure.getUriFor(Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES),
            false, accessibilityObserver,
        )
        cr.registerContentObserver(
            Settings.Secure.getUriFor("accessibility_button_targets"),
            false, accessibilityObserver,
        )
        cr.registerContentObserver(
            Settings.Secure.getUriFor("accessibility_shortcut_target_service"),
            false, accessibilityObserver,
        )
    }

    override fun onPause() {
        contentResolver.unregisterContentObserver(accessibilityObserver)
        super.onPause()
    }

    private fun refreshStats() {
        val words = StatsStore.getWords(this)
        val voiceSeconds = StatsStore.getSeconds(this)
        val voiceMinutes = voiceSeconds / 60L
        val wordsToday = StatsStore.getWordsToday(this)

        if (words <= 0L) {
            statsRow.visibility = View.GONE
            statsActionRow.visibility = View.GONE
            statsEmpty.visibility = View.VISIBLE
            return
        }
        statsRow.visibility = View.VISIBLE
        statsActionRow.visibility = View.VISIBLE
        statsEmpty.visibility = View.GONE

        statsWordsNumber.text = formatThousands(words)
        statsMinutesNumber.text = voiceMinutes.toString()

        // «Сегодня» line only materialises after the user has dictated
        // something today — empty days should not shout "Сегодня 0 слов".
        if (wordsToday > 0L) {
            statsToday.text = resources.getQuantityString(
                R.plurals.main_stats_today,
                wordsToday.toPluralSelector(),
                formatThousands(wordsToday),
            )
            statsToday.visibility = View.VISIBLE
        } else {
            statsToday.visibility = View.GONE
        }
    }

    // Russian plural rule branches on last-digit AND last-two-digits, both
    // well-defined for any Long. Cast to Int is safe because we only need
    // the low-order digits to select the bucket — the formatted number is
    // passed separately as a string.
    private fun Long.toPluralSelector(): Int =
        (this % 100L).toInt().let { if (it < 0) -it else it }

    private fun shareAppLink() {
        val totalWords = StatsStore.getWords(this)
        val wordsPhrase = resources.getQuantityString(
            R.plurals.words_count,
            totalWords.toPluralSelector(),
            formatThousands(totalWords),
        )
        val text = getString(R.string.main_share_stats_fmt, wordsPhrase)
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        try {
            startActivity(Intent.createChooser(intent, getString(R.string.main_share_chooser)))
        } catch (_: Exception) {
        }
    }

    private fun formatThousands(value: Long): String {
        // Use a non-breaking space as the thousands separator so "1 234" never
        // wraps mid-number in the narrow two-column layout.
        return if (value < 1000L) value.toString()
        else value.toString().reversed().chunked(3).joinToString("\u00A0").reversed()
    }

    private fun refreshStatuses() {
        val micOk = ContextCompat.checkSelfPermission(
            this, Manifest.permission.RECORD_AUDIO
        ) == PackageManager.PERMISSION_GRANTED
        val serviceOk = AccessibilityHelper.isLiteServiceEnabled(this)
        val batteryOk = run {
            val pm = getSystemService(Context.POWER_SERVICE) as? PowerManager
            pm?.isIgnoringBatteryOptimizations(packageName) == true
        }
        // The accessibility shortcut places an extra small floating button next
        // to our bubble that disables the service when tapped — count it as a
        // setup problem so the headline and promo card honestly reflect state.
        val shortcutOn = AccessibilityHelper.isLiteShortcutEnabled(this)
        val allOk = micOk && serviceOk && batteryOk && !shortcutOn

        // Only surface the shortcut card when the service is on — otherwise
        // the user is still in "enable me first" mode and the extra advisory
        // would be noise.
        shortcutCard.visibility = if (serviceOk && shortcutOn) View.VISIBLE else View.GONE

        when {
            showJustFinished && allOk -> {
                headline.setText(R.string.main_just_finished)
                subline.setText(R.string.main_just_finished_sub)
            }
            allOk -> {
                headline.setText(R.string.main_ready)
                subline.setText(R.string.main_hint)
            }
            else -> {
                headline.setText(R.string.main_needs_setup)
                subline.setText(R.string.main_needs_setup_sub)
            }
        }

        cardMicMissing.visibility = if (micOk) View.GONE else View.VISIBLE
        cardServiceMissing.visibility = if (serviceOk) View.GONE else View.VISIBLE
        cardBatteryMissing.visibility = if (batteryOk) View.GONE else View.VISIBLE
        // Stats and promo both hide while there are setup problems, so the
        // screen doesn't split attention between "fix this" cards and nice-
        // to-have surfaces. Matches the "needs setup" headline state.
        statsCard.visibility = if (allOk) View.VISIBLE else View.GONE
        promoCard.visibility = if (allOk) View.VISIBLE else View.GONE
    }

    private fun openAppDetails() {
        val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
            data = Uri.fromParts("package", packageName, null)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        try {
            startActivity(intent)
        } catch (_: Exception) {
        }
    }

    @SuppressWarnings("BatteryLife")
    private fun requestIgnoreBatteryOptimizations() {
        val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
            data = Uri.fromParts("package", packageName, null)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        try {
            startActivity(intent)
        } catch (_: Exception) {
            // Fallback to plain battery settings if the direct dialog isn't handled.
            try {
                startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            } catch (_: Exception) {
            }
        }
    }

    companion object {
        const val EXTRA_JUST_FINISHED = "just_finished"
        private const val PREFS = "govorun_lite_prefs"
    }
}
