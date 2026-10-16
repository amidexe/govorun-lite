package com.govorun.lite.ui

import android.os.Bundle
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.govorun.lite.R
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductLongTextScreen

internal fun openOnlineProcessingDocuments(activity: Activity) {
    try {
        activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://govorun.club/legal")))
    } catch (_: ActivityNotFoundException) {
        activity.startActivity(Intent(activity, OnlineProcessingTermsActivity::class.java))
    }
}

class OnlineProcessingTermsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val terms = readTerms()
        setContent {
            GovorunTheme {
                ProductLongTextScreen(
                    title = getString(R.string.online_processing_terms_title),
                    intro = getString(R.string.online_processing_terms_intro),
                    introSectionTitle = "О функции",
                    bodySectionTitle = "Полные условия",
                    body = terms,
                    onBack = ::finish,
                )
            }
        }
    }

    private fun readTerms(): String = try {
        // Keep the accepted version bundled so consent remains readable offline.
        // The canonical document centre is https://govorun.club/legal.
        assets.open("online_processing_terms_ru.txt").bufferedReader().use { it.readText() }
    } catch (_: Exception) {
        getString(R.string.online_processing_terms_missing)
    }
}
