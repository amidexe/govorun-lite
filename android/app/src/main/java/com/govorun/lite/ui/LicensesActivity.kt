package com.govorun.lite.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductLongTextScreen

class LicensesActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val licenses = readLicenses()
        setContent {
            GovorunTheme {
                ProductLongTextScreen(
                    title = "Лицензии",
                    bodySectionTitle = "Компоненты с открытым кодом",
                    body = licenses,
                    onBack = ::finish,
                )
            }
        }
    }

    private fun readLicenses(): String = try {
        assets.open("third_party_licenses.txt").bufferedReader().use { it.readText() }
    } catch (_: Exception) {
        "third_party_licenses.txt not found"
    }
}
