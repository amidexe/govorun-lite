package com.govorun.lite.ui

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.govorun.lite.R
import com.govorun.lite.ui.design.GovorunTheme
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productScreenPadding

class OurProductsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            androidx.compose.runtime.LaunchedEffect(Unit) { ProductNotice.viewed(this@OurProductsActivity) }
            GovorunTheme {
                OurProductsScreen(onBack = ::finish, onEcho = ::openEcho)
            }
        }
    }

    private fun openEcho() {
        try {
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(getString(R.string.echo_product_url))))
        } catch (_: ActivityNotFoundException) {
            Toast.makeText(this, R.string.echo_product_link_error, Toast.LENGTH_SHORT).show()
        }
    }
}

@Composable
private fun OurProductsScreen(onBack: () -> Unit, onEcho: () -> Unit) {
    ProductScaffold(title = "Наши продукты", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                EchoProductCard(onClick = onEcho)
            }
        }
    }
}
