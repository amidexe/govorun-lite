package com.govorun.lite.account

import android.app.Activity
import android.content.Intent
import com.govorun.lite.ui.MainActivity
import com.govorun.lite.ui.SubscriptionActivity

/** The device-limit branch must finish at the same destination as ordinary OAuth. */
internal fun finishAccountLogin(activity: Activity) {
    activity.startActivity(accountLoginDestinationIntent(activity, AccountLoginReturnHandoff.consume(activity)))
    activity.finish()
}

internal fun accountLoginDestinationIntent(context: android.content.Context, destination: AccountLoginDestination): Intent =
    Intent(context, if (destination == AccountLoginDestination.SUBSCRIPTION) SubscriptionActivity::class.java else MainActivity::class.java)
        .putExtra(MainActivity.EXTRA_CONTINUE_STYLIZATION, destination == AccountLoginDestination.STYLIZATION)
        .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)

internal fun shouldFinishAccountLogin(explicitLogin: Boolean, state: AccountState): Boolean =
    explicitLogin && state.signedIn && !state.syncing && !state.deviceLimitReached &&
        state.secureStorageProblem == null && state.dataSyncMode != AccountDataSyncMode.EVALUATING
