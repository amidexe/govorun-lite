package com.govorun.lite.notifications

import android.app.PendingIntent
import android.content.Intent
import android.net.Uri
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.govorun.lite.R

class GovorunMessagingService : FirebaseMessagingService() {
    override fun handleIntent(intent: Intent) {
        // Reject Firebase Console's automatic notification payload before the
        // SDK can display it in the background without our category checks.
        // Supported delivery always uses the validated data-only envelope.
        if (intent.extras?.let { RemoteMessage(it).notification != null } == true) return
        super.handleIntent(intent)
    }

    override fun onNewToken(token: String) {
        // The SDK owns the token. A refresh rereads it only while consent is on;
        // the callback never logs it or registers an installation by itself.
        NotificationRepository.refresh()
    }

    override fun onMessageReceived(message: RemoteMessage) {
        val notice = NotificationRepository.receive(message.data) ?: return
        NotificationRepository.createChannels()
        val pending = PendingIntent.getActivity(this, if (notice.category == "updates") 8101 else 8102,
            Intent(Intent.ACTION_VIEW, Uri.parse(if (notice.category == "updates")
                getString(R.string.main_pro_review_url) else "https://govorun.club")),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val notification = NotificationCompat.Builder(this, "govorun_messages")
            .setSmallIcon(R.drawable.ic_bird_24).setContentTitle(notice.title).setContentText(notice.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(notice.body)).setContentIntent(pending)
            .setAutoCancel(true).setOnlyAlertOnce(true).setSilent(true)
            .setTimeoutAfter((notice.expiresAt * 1000 - System.currentTimeMillis()).coerceAtLeast(1))
            .setCategory(NotificationCompat.CATEGORY_STATUS).build()
        // Serialize with local revocation so it cannot race notification posting.
        synchronized(NotificationRepository) {
            if (NotificationRepository.mayShow(notice)) {
                try { NotificationManagerCompat.from(this).notify(notice.category, 8101, notification) }
                catch (_: SecurityException) { NotificationRepository.refresh() }
            }
        }
    }
}
