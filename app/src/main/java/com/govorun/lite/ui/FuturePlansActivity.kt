package com.govorun.lite.ui

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.View
import android.widget.ImageView
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButton
import com.google.android.material.color.DynamicColors
import com.google.android.material.textview.MaterialTextView
import com.govorun.lite.R

/**
 * "Что будет дальше" — the future-features teaser with a RuStore review CTA.
 * Accessed via the MainActivity toolbar overflow menu.
 */
class FuturePlansActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        DynamicColors.applyToActivityIfAvailable(this)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_future_plans)

        val toolbar = findViewById<MaterialToolbar>(R.id.topAppBar)
        toolbar.setNavigationOnClickListener { finish() }

        val scroll = findViewById<View>(R.id.scroll)
        ViewCompat.setOnApplyWindowInsetsListener(scroll) { v, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.navigationBars() or WindowInsetsCompat.Type.displayCutout()
            )
            v.updatePadding(bottom = bars.bottom)
            insets
        }

        bindFeatureRow(R.id.rowBgRecord, R.drawable.ic_check_circle_24,
            R.string.main_pro_feat_bg_record_title, R.string.main_pro_feat_bg_record_body)
        bindFeatureRow(R.id.rowDict, R.drawable.ic_check_circle_24,
            R.string.main_pro_feat_dict_title, R.string.main_pro_feat_dict_body)
        bindFeatureRow(R.id.rowTheme, R.drawable.ic_check_circle_24,
            R.string.main_pro_feat_theme_title, R.string.main_pro_feat_theme_body)
        bindFeatureRow(R.id.rowFilter, R.drawable.ic_check_circle_24,
            R.string.main_pro_feat_filter_title, R.string.main_pro_feat_filter_body)

        findViewById<MaterialButton>(R.id.proReviewButton).setOnClickListener {
            openRuStoreReview()
        }
        findViewById<MaterialButton>(R.id.proShareButton).setOnClickListener {
            shareApp()
        }
    }

    private fun bindFeatureRow(rowId: Int, iconRes: Int, titleRes: Int, bodyRes: Int) {
        val row = findViewById<View>(rowId)
        row.findViewById<ImageView>(R.id.rowIcon).setImageResource(iconRes)
        row.findViewById<MaterialTextView>(R.id.rowTitle).setText(titleRes)
        row.findViewById<MaterialTextView>(R.id.rowBody).setText(bodyRes)
    }

    private fun openRuStoreReview() {
        val uri = Uri.parse(getString(R.string.main_pro_review_url))
        val intent = Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            // No handler — silently no-op.
        }
    }

    private fun shareApp() {
        val text = getString(R.string.main_share_app_text)
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        try {
            startActivity(Intent.createChooser(intent, getString(R.string.main_share_chooser)))
        } catch (_: Exception) {
        }
    }
}
