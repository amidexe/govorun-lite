package com.govorun.lite.ui

import android.content.DialogInterface
import android.content.Intent
import android.os.Bundle
import kotlin.math.roundToInt
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButton
import com.google.android.material.button.MaterialButtonToggleGroup
import com.google.android.material.color.DynamicColors
import com.google.android.material.color.MaterialColors
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.android.material.materialswitch.MaterialSwitch
import com.google.android.material.slider.Slider
import com.google.android.material.textview.MaterialTextView
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.service.LiteAccessibilityService
import com.govorun.lite.util.AccessibilityHelper
import com.govorun.lite.util.Prefs

class SettingsActivity : AppCompatActivity() {

    private lateinit var showServiceRow: View
    private lateinit var showServiceSwitch: MaterialSwitch
    private lateinit var appFilterSubtitle: MaterialTextView

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        DynamicColors.applyToActivityIfAvailable(this)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)

        findViewById<MaterialToolbar>(R.id.topAppBar)
            .setNavigationOnClickListener { finish() }

        val scroll = findViewById<View>(R.id.scroll)
        ViewCompat.setOnApplyWindowInsetsListener(scroll) { v, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.navigationBars() or WindowInsetsCompat.Type.displayCutout()
            )
            v.updatePadding(bottom = bars.bottom)
            insets
        }

        // Preview bubble — real BubbleView right in Settings, so slider
        // changes are visible immediately without opening a chat. Initial
        // alpha is read here so the first paint matches stored prefs;
        // size/scale comes from BubbleView's own Prefs read in init.
        val previewBubble = findViewById<BubbleView>(R.id.previewBubble)
        previewBubble.setIdleAlpha(Prefs.getBubbleAlpha(this))

        val sideGroup = findViewById<MaterialButtonToggleGroup>(R.id.sideToggleGroup)
        // Pre-check the button matching the stored side BEFORE wiring the
        // listener so the initial state doesn't fire a side-change event.
        val initialSideId = if (Prefs.getBubbleSide(this) == Prefs.BUBBLE_SIDE_LEFT)
            R.id.sideLeft else R.id.sideRight
        sideGroup.check(initialSideId)
        applyPreviewSide(previewBubble, Prefs.getBubbleSide(this))
        applyPreviewEdgeMargin(previewBubble, Prefs.getBubbleSide(this), Prefs.getBubbleEdgeMargin(this))
        sideGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (!isChecked) return@addOnButtonCheckedListener
            val side = if (checkedId == R.id.sideLeft) Prefs.BUBBLE_SIDE_LEFT
                       else Prefs.BUBBLE_SIDE_RIGHT
            Prefs.setBubbleSide(this, side)
            applyPreviewSide(previewBubble, side)
            applyPreviewEdgeMargin(previewBubble, side, Prefs.getBubbleEdgeMargin(this))
            LiteAccessibilityService.instance?.applyBubbleSideFromPrefs()
        }

        val sizeSlider = findViewById<Slider>(R.id.sizeSlider)
        sizeSlider.valueFrom = Prefs.BUBBLE_SIZE_MIN
        sizeSlider.valueTo = Prefs.BUBBLE_SIZE_MAX
        sizeSlider.value = Prefs.getBubbleSize(this)
        // Show the slider label as a percent of the base size (1.0 = 100%)
        // instead of the raw float — it's the universal pattern for these
        // kinds of UI controls and reads better than "0.85".
        sizeSlider.setLabelFormatter { value -> "${(value * 100).roundToInt()}%" }
        sizeSlider.addOnChangeListener { _, value, fromUser ->
            if (!fromUser) return@addOnChangeListener
            Prefs.setBubbleSize(this, value)
            previewBubble.applySizeScaleFromPrefs()
            // Size change requires a relayout — simplest is to rebuild the
            // bubble view, same as how wallpaper-colour changes are handled.
            LiteAccessibilityService.instance?.applyBubbleSizeFromPrefs()
        }

        val transparencySlider = findViewById<Slider>(R.id.transparencySlider)
        transparencySlider.valueFrom = Prefs.BUBBLE_ALPHA_MIN
        transparencySlider.valueTo = Prefs.BUBBLE_ALPHA_MAX
        transparencySlider.value = Prefs.getBubbleAlpha(this)
        // Same percent formatting — alpha 0.857 → "86%", much easier to
        // read than "0.857" or "0.142857".
        transparencySlider.setLabelFormatter { value -> "${(value * 100).roundToInt()}%" }
        transparencySlider.addOnChangeListener { _, value, fromUser ->
            if (!fromUser) return@addOnChangeListener
            Prefs.setBubbleAlpha(this, value)
            previewBubble.setIdleAlpha(value)
            // Service might be off right now (onboarding not finished, or user
            // disabled it). If it's on, nudge it so the overlay bubble reflects
            // the change immediately — no need to toggle or reopen anything.
            LiteAccessibilityService.instance?.applyBubbleAlphaFromPrefs()
        }

        val edgeMarginSlider = findViewById<Slider>(R.id.edgeMarginSlider)
        edgeMarginSlider.valueFrom = Prefs.BUBBLE_EDGE_MARGIN_MIN.toFloat()
        edgeMarginSlider.valueTo = Prefs.BUBBLE_EDGE_MARGIN_MAX.toFloat()
        edgeMarginSlider.stepSize = Prefs.BUBBLE_EDGE_MARGIN_STEP.toFloat()
        edgeMarginSlider.value = Prefs.getBubbleEdgeMargin(this).toFloat()
        edgeMarginSlider.setLabelFormatter { value -> "${value.toInt()} dp" }
        edgeMarginSlider.addOnChangeListener { _, value, fromUser ->
            if (!fromUser) return@addOnChangeListener
            val marginDp = value.toInt()
            Prefs.setBubbleEdgeMargin(this, marginDp)
            applyPreviewEdgeMargin(previewBubble, Prefs.getBubbleSide(this), marginDp)
            LiteAccessibilityService.instance?.applyBubbleEdgeMarginFromPrefs()
        }

        val hapticsSwitch = findViewById<MaterialSwitch>(R.id.hapticsSwitch)
        val hapticsRow = findViewById<View>(R.id.hapticsRow)

        hapticsSwitch.isChecked = Prefs.isHapticsEnabled(this)
        hapticsSwitch.setOnCheckedChangeListener { _, checked ->
            Prefs.setHapticsEnabled(this, checked)
        }
        hapticsRow.setOnClickListener { hapticsSwitch.toggle() }

        val hideInSearchSwitch = findViewById<MaterialSwitch>(R.id.hideInSearchSwitch)
        val hideInSearchRow = findViewById<View>(R.id.hideInSearchRow)

        hideInSearchSwitch.isChecked = Prefs.isHideInSearchEnabled(this)
        hideInSearchSwitch.setOnCheckedChangeListener { _, checked ->
            Prefs.setHideInSearchEnabled(this, checked)
        }
        hideInSearchRow.setOnClickListener { hideInSearchSwitch.toggle() }

        val bubbleLockSwitch = findViewById<MaterialSwitch>(R.id.bubbleLockSwitch)
        val bubbleLockRow = findViewById<View>(R.id.bubbleLockRow)

        bubbleLockSwitch.isChecked = Prefs.isBubblePositionLocked(this)
        bubbleLockSwitch.setOnCheckedChangeListener { _, checked ->
            Prefs.setBubblePositionLocked(this, checked)
        }
        bubbleLockRow.setOnClickListener { bubbleLockSwitch.toggle() }

        val voiceSuffixSwitch = findViewById<MaterialSwitch>(R.id.voiceSuffixSwitch)
        val voiceSuffixRow = findViewById<View>(R.id.voiceSuffixRow)

        voiceSuffixSwitch.isChecked = Prefs.isVoiceSuffixEnabled(this)
        voiceSuffixSwitch.setOnCheckedChangeListener { _, checked ->
            Prefs.setVoiceSuffixEnabled(this, checked)
        }
        voiceSuffixRow.setOnClickListener { voiceSuffixSwitch.toggle() }

        val keepScreenSwitch = findViewById<MaterialSwitch>(R.id.keepScreenSwitch)
        val keepScreenRow = findViewById<View>(R.id.keepScreenRow)
        val keepScreenBody = findViewById<MaterialTextView>(R.id.keepScreenBody)
        val autoStopRow = findViewById<View>(R.id.autoStopRow)
        val autoStopGroup = findViewById<MaterialButtonToggleGroup>(R.id.autoStopToggleGroup)

        // Show/hide the auto-stop picker based on the keep-screen-on toggle.
        // When the toggle is OFF the system screen timeout already kills the
        // session via screenOffReceiver well before any wall-clock cap could
        // fire, so the picker would just be confusing dead UI.
        fun applyAutoStopVisibility(keepScreenOn: Boolean) {
            autoStopRow.visibility = if (keepScreenOn) View.VISIBLE else View.GONE
        }
        fun applyKeepScreenBody(checked: Boolean) {
            keepScreenBody.setText(
                if (checked) R.string.settings_keep_screen_body_on
                else R.string.settings_keep_screen_body_off
            )
        }

        keepScreenSwitch.isChecked = Prefs.isKeepScreenOnEnabled(this)
        applyKeepScreenBody(keepScreenSwitch.isChecked)
        applyAutoStopVisibility(keepScreenSwitch.isChecked)
        keepScreenSwitch.setOnCheckedChangeListener { _, checked ->
            Prefs.setKeepScreenOnEnabled(this, checked)
            applyKeepScreenBody(checked)
            applyAutoStopVisibility(checked)
        }
        keepScreenRow.setOnClickListener { keepScreenSwitch.toggle() }

        autoStopGroup.check(when (Prefs.getAutoStopMinutes(this)) {
            Prefs.AUTO_STOP_15M -> R.id.autoStop15m
            Prefs.AUTO_STOP_3H -> R.id.autoStop3h
            Prefs.AUTO_STOP_OFF -> R.id.autoStopOff
            else -> R.id.autoStop1h
        })
        autoStopGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (!isChecked) return@addOnButtonCheckedListener
            val minutes = when (checkedId) {
                R.id.autoStop15m -> Prefs.AUTO_STOP_15M
                R.id.autoStop3h -> Prefs.AUTO_STOP_3H
                R.id.autoStopOff -> Prefs.AUTO_STOP_OFF
                else -> Prefs.AUTO_STOP_1H
            }
            Prefs.setAutoStopMinutes(this, minutes)
        }

        // Pause length — VAD silence threshold preset. "Короткая" preselects
        // for existing users (matches what shipped before 1.0.7); their
        // experience is unchanged unless they actively switch.
        val pauseGroup = findViewById<MaterialButtonToggleGroup>(R.id.pauseToggleGroup)
        val pauseHint = findViewById<MaterialTextView>(R.id.pauseHint)
        val initialPause = Prefs.getPauseLength(this)
        pauseGroup.check(when (initialPause) {
            Prefs.PAUSE_MEDIUM -> R.id.pauseMedium
            Prefs.PAUSE_LONG -> R.id.pauseLong
            else -> R.id.pauseShort
        })
        pauseHint.setText(hintForPause(initialPause))
        pauseGroup.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (!isChecked) return@addOnButtonCheckedListener
            val value = when (checkedId) {
                R.id.pauseMedium -> Prefs.PAUSE_MEDIUM
                R.id.pauseLong -> Prefs.PAUSE_LONG
                else -> Prefs.PAUSE_SHORT
            }
            Prefs.setPauseLength(this, value)
            pauseHint.setText(hintForPause(value))
        }

        findViewById<View>(R.id.dictionaryRow).setOnClickListener {
            startActivity(Intent(this, DictionaryActivity::class.java))
        }

        appFilterSubtitle = findViewById(R.id.appFilterSubtitle)
        findViewById<View>(R.id.appFilterRow).setOnClickListener {
            startActivity(Intent(this, AppFilterActivity::class.java))
        }

        findViewById<View>(R.id.benchmarkRow).setOnClickListener {
            startActivity(Intent(this, BenchmarkActivity::class.java))
        }

        showServiceRow = findViewById(R.id.showServiceRow)
        showServiceSwitch = findViewById(R.id.showServiceSwitch)
        // Row is visible regardless of service state so the user always has a
        // discoverable way back into accessibility settings. The tap handler
        // branches on the current service state: on → confirm-disable dialog,
        // off → "go turn it on again" dialog. Hiding the row when the service
        // is off would leave the user wondering where the setting went, with
        // no in-app breadcrumb to the system screen.
        showServiceRow.setOnClickListener {
            if (AccessibilityHelper.isLiteServiceEnabled(this)) {
                confirmDisableService()
            } else {
                promptEnableService()
            }
        }

        findViewById<MaterialButton>(R.id.resetButton).setOnClickListener {
            // Reset all customisation prefs to project defaults. Sliders and
            // switch update via setValue/isChecked (no fromUser flag → our
            // addOnChangeListener sees fromUser=false and skips the side-
            // effects), so we apply preview + overlay changes manually after.
            // Dictionary text + enabled flag are NOT reset — wiping the user's
            // hand-curated word list from a generic "Reset" button would be
            // surprising; the dictionary screen has its own clear action.
            Prefs.setBubbleSize(this, Prefs.BUBBLE_SIZE_DEFAULT)
            Prefs.setBubbleAlpha(this, Prefs.BUBBLE_ALPHA_DEFAULT)
            Prefs.setBubbleEdgeMargin(this, Prefs.BUBBLE_EDGE_MARGIN_DEFAULT)
            Prefs.setHapticsEnabled(this, false)
            Prefs.setHideInSearchEnabled(this, false)
            Prefs.setBubbleSide(this, Prefs.BUBBLE_SIDE_RIGHT)
            // Reset Y too — "factory defaults" should put the bubble back to
            // the centre. A user who deliberately positioned it isn't going
            // to hit Reset; anyone who does expects everything reverted.
            Prefs.setBubbleY(this, 0)
            Prefs.setPauseLength(this, Prefs.PAUSE_DEFAULT)
            Prefs.setKeepScreenOnEnabled(this, false)
            Prefs.setAutoStopMinutes(this, Prefs.AUTO_STOP_DEFAULT)

            sizeSlider.value = Prefs.BUBBLE_SIZE_DEFAULT
            transparencySlider.value = Prefs.BUBBLE_ALPHA_DEFAULT
            edgeMarginSlider.value = Prefs.BUBBLE_EDGE_MARGIN_DEFAULT.toFloat()
            hapticsSwitch.isChecked = false
            hideInSearchSwitch.isChecked = false
            sideGroup.check(R.id.sideRight)
            pauseGroup.check(R.id.pauseShort)
            pauseHint.setText(hintForPause(Prefs.PAUSE_DEFAULT))
            keepScreenSwitch.isChecked = false
            applyKeepScreenBody(false)
            applyAutoStopVisibility(false)
            autoStopGroup.check(R.id.autoStop1h)

            previewBubble.applySizeScaleFromPrefs()
            previewBubble.setIdleAlpha(Prefs.BUBBLE_ALPHA_DEFAULT)
            applyPreviewSide(previewBubble, Prefs.BUBBLE_SIDE_RIGHT)
            applyPreviewEdgeMargin(previewBubble, Prefs.BUBBLE_SIDE_RIGHT, Prefs.BUBBLE_EDGE_MARGIN_DEFAULT)

            LiteAccessibilityService.instance?.applyBubbleSizeFromPrefs()
            LiteAccessibilityService.instance?.applyBubbleAlphaFromPrefs()
            LiteAccessibilityService.instance?.applyBubbleSideFromPrefs()
            LiteAccessibilityService.instance?.applyBubbleEdgeMarginFromPrefs()
            // Position reset — a fresh bubble rebuild picks up the new Y=0.
            LiteAccessibilityService.instance?.applyBubbleSizeFromPrefs()
        }
    }

    override fun onResume() {
        super.onResume()
        refreshServiceSwitch()
        updateAppFilterSubtitle()
    }

    /**
     * Push the preview bubble to the chosen side of its FrameLayout, so the
     * Settings preview visually mirrors what the user picked. Uses the
     * BubbleView's layout_gravity within its parent.
     */
    private fun hintForPause(value: String): Int = when (value) {
        Prefs.PAUSE_MEDIUM -> R.string.settings_pause_hint_medium
        Prefs.PAUSE_LONG -> R.string.settings_pause_hint_long
        else -> R.string.settings_pause_hint_short
    }

    private fun applyPreviewSide(previewBubble: BubbleView, side: String) {
        val lp = previewBubble.layoutParams as? android.widget.FrameLayout.LayoutParams ?: return
        val horizontal = if (side == Prefs.BUBBLE_SIDE_LEFT) android.view.Gravity.START
                         else android.view.Gravity.END
        lp.gravity = horizontal or android.view.Gravity.CENTER_VERTICAL
        previewBubble.layoutParams = lp
    }

    // Shifts the preview bubble horizontally to mirror the real overlay x offset.
    // Right side: positive margin = move left (away from edge) → negative translationX.
    // Left side:  positive margin = move right (away from edge) → positive translationX.
    private fun applyPreviewEdgeMargin(previewBubble: BubbleView, side: String, marginDp: Int) {
        val px = marginDp * resources.displayMetrics.density
        previewBubble.translationX = if (side == Prefs.BUBBLE_SIDE_LEFT) px else -px
    }

    private fun refreshServiceSwitch() {
        val enabled = AccessibilityHelper.isLiteServiceEnabled(this)
        showServiceSwitch.isChecked = enabled
        // Title shows current state ("включён" / "выключен"); body says what
        // tapping the row will do. Without this dynamic update the row reads
        // as "Показывать Говоруна" with a switch — and tapping it confusingly
        // brings up a "выключить?" dialog when the user expected the switch
        // to just flip.
        findViewById<MaterialTextView>(R.id.showServiceTitle).setText(
            if (enabled) R.string.settings_service_on_title
            else R.string.settings_service_off_title
        )
        findViewById<MaterialTextView>(R.id.showServiceBody).setText(
            if (enabled) R.string.settings_service_on_body
            else R.string.settings_service_off_body
        )
    }

    private fun confirmDisableService() {
        val dialog = MaterialAlertDialogBuilder(this)
            .setTitle(R.string.main_disable_service_title)
            .setMessage(R.string.main_disable_service_hint)
            .setPositiveButton(R.string.main_disable_service) { _, _ ->
                LiteAccessibilityService.instance?.disableSelf()
                // System updates the enabled list asynchronously; refresh after
                // a beat so the switch reflects reality.
                showServiceRow.postDelayed({ refreshServiceSwitch() }, 300)
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
        // Tint the destructive action so the «Выключить» button reads as a
        // consequence, not a casual OK. M3 doesn't ship a destructive-button
        // style out of the box, so we recolour it after show().
        dialog.getButton(DialogInterface.BUTTON_POSITIVE)?.setTextColor(
            MaterialColors.getColor(showServiceRow, com.google.android.material.R.attr.colorError)
        )
    }

    private fun promptEnableService() {
        MaterialAlertDialogBuilder(this)
            .setTitle(R.string.main_enable_service_title)
            .setMessage(R.string.main_enable_service_body)
            .setPositiveButton(R.string.main_open_accessibility) { _, _ ->
                AccessibilityHelper.openAccessibilitySettings(this)
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun updateAppFilterSubtitle() {
        val mode = Prefs.getAppFilterMode(this)
        val count = Prefs.getAppFilterPackages(this).size
        appFilterSubtitle.text = when {
            count == 0 -> getString(R.string.settings_app_filter_body_all)
            mode == Prefs.APP_FILTER_WHITELIST ->
                getString(R.string.settings_app_filter_body_whitelist, count)
            else ->
                getString(R.string.settings_app_filter_body_blacklist, count)
        }
    }
}
