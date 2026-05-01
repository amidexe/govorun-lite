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
import com.google.android.material.color.DynamicColors
import com.google.android.material.textview.MaterialTextView
import com.govorun.lite.BuildConfig
import com.govorun.lite.R

class AboutActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        DynamicColors.applyToActivityIfAvailable(this)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_about)

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

        val version = BuildConfig.VERSION_NAME
        findViewById<MaterialTextView>(R.id.aboutVersion).text =
            getString(R.string.about_version_fmt, version)
        findViewById<MaterialTextView>(R.id.aboutFooter).text =
            getString(R.string.about_footer_fmt, version)

        bindInfoRow(
            R.id.rowEngine, R.drawable.ic_bird_24,
            R.string.about_row_engine_title, R.string.about_row_engine_body
        )
        bindInfoRow(
            R.id.rowOffline, R.drawable.ic_shield_24,
            R.string.about_row_offline_title, R.string.about_row_offline_body
        )
        bindInfoRow(
            R.id.rowSource, R.drawable.ic_code_24,
            R.string.about_row_source_title, R.string.about_row_source_body
        )
        bindInfoRow(
            R.id.rowUpdates, R.drawable.ic_update_24,
            R.string.about_row_updates_title, R.string.about_row_updates_body
        )

        val github = findViewById<View>(R.id.rowGithub)
        bindActionRow(
            github, R.drawable.ic_github_24,
            R.string.about_action_github_title, R.string.about_action_github_sub
        )
        github.setOnClickListener { openExternal(getString(R.string.about_github_url)) }

        val licenses = findViewById<View>(R.id.rowLicenses)
        bindActionRow(
            licenses, R.drawable.ic_gavel_24,
            R.string.about_action_licenses_title, R.string.about_action_licenses_sub
        )
        licenses.setOnClickListener {
            startActivity(Intent(this, LicensesActivity::class.java))
        }

        val privacy = findViewById<View>(R.id.rowPrivacy)
        bindActionRow(
            privacy, R.drawable.ic_shield_24,
            R.string.about_action_privacy_title, R.string.about_action_privacy_sub
        )
        privacy.setOnClickListener { openExternal(getString(R.string.about_privacy_url)) }
    }

    private fun bindInfoRow(rowId: Int, iconRes: Int, titleRes: Int, bodyRes: Int) {
        val row = findViewById<View>(rowId)
        row.findViewById<ImageView>(R.id.rowIcon).setImageResource(iconRes)
        row.findViewById<MaterialTextView>(R.id.rowTitle).setText(titleRes)
        row.findViewById<MaterialTextView>(R.id.rowBody).setText(bodyRes)
    }

    private fun bindActionRow(row: View, iconRes: Int, titleRes: Int, bodyRes: Int) {
        row.findViewById<ImageView>(R.id.rowIcon).setImageResource(iconRes)
        row.findViewById<MaterialTextView>(R.id.rowTitle).setText(titleRes)
        row.findViewById<MaterialTextView>(R.id.rowBody).setText(bodyRes)
    }

    private fun openExternal(url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            startActivity(intent)
        } catch (_: ActivityNotFoundException) {
        }
    }
}
