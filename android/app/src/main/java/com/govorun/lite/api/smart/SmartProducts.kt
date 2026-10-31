package com.govorun.lite.api.smart

import android.content.Context
import com.govorun.lite.stylization.ProductLabel

/**
 * Govorun's products besides dictation and the apps they serve. Govorun labels an app's request with the app's product once, at
 * the entry, from the app it knows by package and signature, never from what the app sends; the core admits and counts it under
 * that product's own line. A new product is its label and its app here, its limits on the server.
 */
internal object SmartProducts {
    /** The product an app's requests are for; null for an app with no product of its own. */
    fun of(packageName: String): String? = if (EchoApp.matches(packageName)) ProductLabel.ECHO else null

    /** The person let the product's app use Govorun: only then does Govorun's account show the product's own limit. */
    fun connected(context: Context, product: String): Boolean = when (product) {
        ProductLabel.ECHO -> EchoApp.connected(context)
        else -> false
    }
}
