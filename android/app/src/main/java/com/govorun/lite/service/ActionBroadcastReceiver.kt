package com.govorun.lite.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import androidx.core.content.ContextCompat

internal fun Context.registerActionReceiver(receiver: BroadcastReceiver, action: String) {
    ContextCompat.registerReceiver(this, receiver, IntentFilter(action), ContextCompat.RECEIVER_NOT_EXPORTED)
}

/** Creates a receiver that ignores every broadcast except its declared action. */
internal fun actionBroadcastReceiver(
    action: String,
    onReceive: () -> Unit,
): BroadcastReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
        if (intent?.action == action) onReceive()
    }
}
