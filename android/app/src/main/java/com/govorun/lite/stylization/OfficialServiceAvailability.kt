package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.account.GovorunAccountRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** UI projection and recovery event; availability policy and probes live in Core. */
internal object OfficialServiceAvailability {
    private val state = MutableStateFlow(false)
    val paused = state.asStateFlow()
    fun get(@Suppress("UNUSED_PARAMETER") context: Context) = this
    fun project(value: Boolean) { state.value = value }
    fun requestRecovery() = GovorunAccountRepository.recoverStyles()
}
