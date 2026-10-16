package com.govorun.lite.stylization

import com.govorun.lite.account.AccountState
import com.govorun.lite.account.accessWindowInstruction
import com.govorun.lite.account.sameActivationAccount
import com.govorun.lite.account.styleSetupNeedsSubscription
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive

/** Executes the common instruction against captured UI effects; never grants access locally. */
internal suspend fun performStyleAccessInstruction(
    original: AccountState,
    localCurrentTermsAccepted: Boolean,
    currentAccount: () -> AccountState,
    activateAccess: suspend (acceptCurrentTerms: Boolean) -> Boolean,
    acceptTerms: suspend () -> Boolean,
    enableStyle: (recordConsent: Boolean) -> Boolean,
    action: String,
    confirmed: Boolean = false,
): Boolean {
    currentCoroutineContext().ensureActive()
    val before = currentAccount()
    if (!sameActivationAccount(original, before)) return false
    val instruction = accessWindowInstruction(before, localCurrentTermsAccepted, action, confirmed)
    val accepted = when (instruction.kind) {
        "applyLocalAccess" -> true
        "activateFreeAccess" -> activateAccess(instruction.includeTerms)
        "acceptTerms" -> acceptTerms()
        else -> return false
    }
    currentCoroutineContext().ensureActive()
    val current = currentAccount()
    if (!accepted || !sameActivationAccount(original, current)) return false
    val completed = accessWindowInstruction(current,
        localCurrentTermsAccepted || instruction.includeTerms).kind == "applyLocalAccess"
    return completed && !styleSetupNeedsSubscription(current) && enableStyle(instruction.includeTerms)
}
