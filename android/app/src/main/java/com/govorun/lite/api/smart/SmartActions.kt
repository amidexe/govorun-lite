package com.govorun.lite.api.smart

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import com.govorun.lite.api.CallerIdentity
import com.govorun.lite.api.RecognitionApi
import com.govorun.lite.api.RecognitionAuthorizationActivity
import com.govorun.lite.api.RecognitionAuthorizationStore
import com.govorun.lite.network.NetworkPolicy
import com.govorun.lite.network.NetworkRefusal
import com.govorun.lite.ui.AccountActivity
import com.govorun.lite.ui.MainActivity
import com.govorun.lite.ui.SettingsActivity
import com.govorun.lite.ui.SubscriptionActivity

/** Govorun's own screens that settle a state; the person decides there, never the client. */
internal object SmartActions {
    fun create(
        context: Context,
        identity: CallerIdentity,
        action: String,
        authorizations: RecognitionAuthorizationStore,
    ): PendingIntent {
        val intent = when (action) {
            SmartApi.ACTION_SIGN_IN -> Intent(context, AccountActivity::class.java)
            SmartApi.ACTION_SUBSCRIPTION -> Intent(context, SubscriptionActivity::class.java)
            // «Офлайн» lives in the home menu, «Офлайн в роуминге» in Settings, as Govorun's own card does.
            SmartApi.ACTION_STYLIZATION -> Intent(context, com.govorun.lite.ui.StylizationActivity::class.java)
            SmartApi.ACTION_NETWORK -> when (NetworkPolicy.activeRefusal(context)) {
                NetworkRefusal.ROAMING -> Intent(context, SettingsActivity::class.java).putExtra(SettingsActivity.EXTRA_NETWORK, true)
                else -> Intent(context, MainActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
                    .putExtra(MainActivity.EXTRA_OPEN_NETWORK, true)
            }
            SmartApi.ACTION_AUTHORIZE, SmartApi.ACTION_VOLUME_KEY -> {
                val scope = if (action == SmartApi.ACTION_VOLUME_KEY) RecognitionApi.SCOPE_VOLUME_KEY else RecognitionApi.SCOPE_SMART_FUNCTIONS
                val pending = authorizations.createPending(identity, scope)
                return PendingIntent.getActivity(
                    context,
                    0,
                    Intent(context, RecognitionAuthorizationActivity::class.java)
                        .addFlags(
                            Intent.FLAG_ACTIVITY_NEW_TASK or
                                Intent.FLAG_ACTIVITY_NO_HISTORY or
                                Intent.FLAG_ACTIVITY_EXCLUDE_FROM_RECENTS,
                        )
                        .putExtra(RecognitionAuthorizationActivity.EXTRA_NONCE, pending.nonce)
                        .setIdentifier("govorun-$action-auth:${pending.nonce}"),
                    PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_ONE_SHOT,
                )
            }
            else -> throw IllegalArgumentException("Unknown action")
        }
        return PendingIntent.getActivity(
            context,
            0,
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).setIdentifier("govorun-smart:$action"),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
    }
}
