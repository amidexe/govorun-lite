package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.StylizationFallbackReason
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** One process-local presentation frame; never persisted or sent to a server. */
internal object StyleProcessingFeedback {
    enum class Phase { IDLE, PROCESSING, FAILED }
    data class Frame(val revision: Long = 0, val generation: Long = -1,
                     val selfHosted: Boolean = false, val phase: Phase = Phase.IDLE,
                     val failureMessage: String? = null)
    data class Ticket(val frame: Frame, val previousFailure: String?)
    private val mutable = MutableStateFlow(Frame())
    val state = mutable.asStateFlow()
    @Synchronized fun begin(generation: Long, selfHosted: Boolean): Ticket {
        val previous = mutable.value
        val frame = Frame(previous.revision + 1, generation, selfHosted, Phase.PROCESSING)
        mutable.value = frame
        return Ticket(frame, previous.failureMessage.takeIf {
            previous.generation == generation && previous.selfHosted == selfHosted
        })
    }
    fun finish(ticket: Ticket, reason: StylizationFallbackReason?) = complete(ticket, when (reason) {
        StylizationFallbackReason.OFFLINE -> "Не было подключения к интернету"
        StylizationFallbackReason.TRANSPORT_FAILED -> "Не удалось связаться с сервисом"
        StylizationFallbackReason.SERVER_BUSY -> "Сервис был занят"
        StylizationFallbackReason.SERVER_TIMEOUT -> "Сервис не ответил вовремя"
        StylizationFallbackReason.SERVER_UNAVAILABLE -> "Сервис был временно недоступен"
        StylizationFallbackReason.STYLE_PASS_ISSUE_FAILED -> "Не удалось подготовить улучшение"
        StylizationFallbackReason.REQUEST_FAILED,
        StylizationFallbackReason.INVALID_RESPONSE -> "Не удалось обработать запрос"
        else -> null
    })
    @Synchronized private fun complete(ticket: Ticket, message: String?) {
        if (mutable.value == ticket.frame) mutable.value = ticket.frame.copy(
            phase = if (message == null) Phase.IDLE else Phase.FAILED, failureMessage = message)
    }
    fun cancel(ticket: Ticket) = complete(ticket, ticket.previousFailure)
    @Synchronized fun dismiss(revision: Long) {
        if (mutable.value.revision == revision && mutable.value.phase == Phase.FAILED)
            mutable.value = mutable.value.copy(phase = Phase.IDLE, failureMessage = null)
    }
    fun snapshot(generation: Long, selfHosted: Boolean): Frame = state.value.let {
        if (it.generation == generation && it.selfHosted == selfHosted) it else Frame(revision = it.revision)
    }
}
