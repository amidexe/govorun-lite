package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository

/** Composition for user edits, with an explicit account synchronization effect. */
internal object AndroidCustomStyles {
    fun create(context: Context): AndroidCustomStyleLibrary {
        val app = context.applicationContext
        return AndroidCustomStyleLibrary(AndroidCustomStyleStorage(app),
            beforeChange = { GovorunAccountRepository.recordSyncedContentChange(app) },
            transaction = { work -> GovorunAccountRepository.withSyncedContentMutation(app, work) })
    }
}
