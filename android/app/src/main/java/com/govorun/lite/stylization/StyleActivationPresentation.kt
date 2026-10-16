package com.govorun.lite.stylization

import com.govorun.lite.account.AccountState
import com.govorun.lite.account.accessWindowInstruction
import com.govorun.lite.account.styleSetupNeedsSubscription
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions

internal fun hasReusableStylizationConsent(account: AccountState, localCurrentTermsAccepted: Boolean): Boolean =
    accessWindowInstruction(account, localCurrentTermsAccepted).reuseConsent

internal fun canReuseStylizationConsent(
    account: AccountState,
    localCurrentTermsAccepted: Boolean,
    accessActivationRequired: Boolean,
): Boolean = !accessActivationRequired &&
    accessWindowInstruction(account, localCurrentTermsAccepted).kind == "applyLocalAccess" &&
    !styleSetupNeedsSubscription(account)

internal fun initialStyleOptions(saved: StyleOptions, firstActivation: Boolean): StyleOptions =
    if (firstActivation) saved.copy(punctuation = PunctuationMode.NORMAL, profanityFilterEnabled = true).withEmojiMode(EmojiMode.LIVELY)
    else saved
