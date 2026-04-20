package com.govorun.lite.ui.onboarding

import android.content.res.ColorStateList
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import com.google.android.material.button.MaterialButton
import com.google.android.material.color.MaterialColors
import com.google.android.material.textview.MaterialTextView
import com.govorun.lite.R
import com.govorun.lite.util.AccessibilityHelper

/**
 * Enabling the LiteAccessibilityService in system settings. Two states:
 * service OFF → show «Открыть настройки»; service ON → step complete. The
 * advisory for Android's auto-enabled shortcut button lives on the main
 * screen (shortcutCard), so this step stays a single-decision flow.
 */
class AccessibilityFragment : OnboardingStepFragment() {

    private lateinit var topIcon: ImageView
    private lateinit var statusText: MaterialTextView
    private lateinit var openButton: MaterialButton
    private lateinit var checklist: View

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View = inflater.inflate(R.layout.fragment_onboarding_accessibility, container, false)

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        topIcon = view.findViewById(R.id.accIcon)
        statusText = view.findViewById(R.id.accStatus)
        openButton = view.findViewById(R.id.accOpen)
        checklist = view.findViewById(R.id.accChecklist)

        openButton.setOnClickListener {
            AccessibilityHelper.openAccessibilitySettings(requireContext())
        }
    }

    override fun onResume() {
        super.onResume()
        refreshState()
    }

    override fun onStepFocused() {
        refreshState()
    }

    private fun refreshState() {
        val ctx = requireContext()
        val serviceOn = AccessibilityHelper.isLiteServiceEnabled(ctx)
        val primary = MaterialColors.getColor(requireView(), com.google.android.material.R.attr.colorPrimary)
        val onSurface = MaterialColors.getColor(requireView(), com.google.android.material.R.attr.colorOnSurface)
        topIcon.imageTintList = ColorStateList.valueOf(primary)
        statusText.setTextColor(onSurface)

        if (serviceOn) {
            statusText.setText(R.string.onb_accessibility_body_enabled)
            openButton.visibility = View.GONE
            checklist.visibility = View.GONE
            setStepComplete(true)
        } else {
            statusText.setText(R.string.onb_accessibility_body_pending)
            openButton.visibility = View.VISIBLE
            checklist.visibility = View.VISIBLE
            setStepComplete(false)
        }
    }
}
