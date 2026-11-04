package com.govorun.lite.stylization

import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext

/** Opaque server operation. All user context stays in the billed text, never in extra unpriced fields. */
internal class TextOperation(val name: String, val requestClass: String) : AbstractCoroutineContextElement(Key) {
    companion object Key : CoroutineContext.Key<TextOperation>
}
