package com.govorun.lite.stylization

import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext

/**
 * Говорун.Эхо's note riding along one official stylization: "note" asks the server to lay the text out as a note, "header" to
 * put a name and a description over the chosen style. Only the Echo entry sets it; dictation never does, so its requests stay
 * exactly as they were. Carried in the coroutine, as [PreparedStylesConnection] is, so the client interface stays as it is.
 */
internal class EchoNoteMark(val value: String) : AbstractCoroutineContextElement(Key) {
    companion object Key : CoroutineContext.Key<EchoNoteMark> {
        const val NOTE = "note"
        const val HEADER = "header"
        /** Echo's итоги of a long note: the server's own task on light's contract, not an improvement. */
        const val DIGEST = "digest"

        /** The style value Echo asks for its «Заметка»: Govorun's light edit, laid out as a note. */
        const val NOTE_STYLE = "note"
        /** The style value of Echo's итоги. */
        const val DIGEST_STYLE = "digest"

        /** Echo's own values go out as light; the server tells them apart by the mark. */
        fun light(styleValue: String) = styleValue == NOTE_STYLE || styleValue == DIGEST_STYLE

        /** The mark for Echo's chosen [styleValue]. */
        fun of(styleValue: String): EchoNoteMark = EchoNoteMark(when (styleValue) {
            NOTE_STYLE -> NOTE
            DIGEST_STYLE -> DIGEST
            else -> HEADER
        })
    }
}
