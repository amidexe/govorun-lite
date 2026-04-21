package com.govorun.lite.ui

import android.content.DialogInterface
import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.color.DynamicColors
import com.google.android.material.color.MaterialColors
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.android.material.materialswitch.MaterialSwitch
import com.google.android.material.slider.Slider
import com.govorun.lite.R
import com.govorun.lite.service.LiteAccessibilityService
import com.govorun.lite.util.AccessibilityHelper
import com.govorun.lite.util.Prefs

class SettingsActivity : AppCompatActivity() {

    private lateinit var serviceSectionHeader: View
    private lateinit var showServiceRow: View
    private lateinit var showServiceSwitch: MaterialSwitch

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

        val transparencySlider = findViewById<Slider>(R.id.transparencySlider)
        transparencySlider.valueFrom = Prefs.BUBBLE_ALPHA_MIN
        transparencySlider.valueTo = Prefs.BUBBLE_ALPHA_MAX
        transparencySlider.value = Prefs.getBubbleAlpha(this)
        transparencySlider.addOnChangeListener { _, value, fromUser ->
            if (!fromUser) return@addOnChangeListener
            Prefs.setBubbleAlpha(this, value)
            // Service might be off right now (onboarding not finished, or user
            // disabled it). If it's on, nudge it so the overlay bubble reflects
            // the change immediately — no need to toggle or reopen anything.
            LiteAccessibilityService.instance?.applyBubbleAlphaFromPrefs()
        }

        val hapticsSwitch = findViewById<MaterialSwitch>(R.id.hapticsSwitch)
        val hapticsRow = findViewById<View>(R.id.hapticsRow)

        hapticsSwitch.isChecked = Prefs.isHapticsEnabled(this)
        hapticsSwitch.setOnCheckedChangeListener { _, checked ->
            Prefs.setHapticsEnabled(this, checked)
        }
        hapticsRow.setOnClickListener { hapticsSwitch.toggle() }

        serviceSectionHeader = findViewById(R.id.serviceSectionHeader)
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
    }

    override fun onResume() {
        super.onResume()
        refreshServiceSwitch()
    }

    private fun refreshServiceSwitch() {
        showServiceSwitch.isChecked = AccessibilityHelper.isLiteServiceEnabled(this)
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
}
