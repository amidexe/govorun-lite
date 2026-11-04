package com.govorun.lite.stylization

import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext

/**
 * The product a stylization is for, labelled once by Govorun's entry: «echo» for Говорун Эхо's notes; Govorun's own dictation
 * carries none. The core admits and counts the request under that product's own line, never under dictation's, and the server
 * sees the label. Carried in the coroutine, as [TextOperation] is, so the client interface stays as it is.
 */
internal class ProductLabel(val value: String) : AbstractCoroutineContextElement(Key) {
    companion object Key : CoroutineContext.Key<ProductLabel> {
        /** Говорун Эхо: its notes, «Улучшить» and итоги. */
        const val ECHO = "echo"

        /** The name a product's line has on Govorun's limits card. */
        fun title(product: String): String = when (product) {
            ECHO -> "Эхо"
            else -> product
        }
    }
}
