package com.govorun.lite.ui.home

import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ArrowCircleUp
import com.adamglin.phosphoricons.regular.X
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.Announcement
import com.govorun.lite.storage.GovorunPreferences
import com.govorun.lite.util.StorageKeys
import com.product.ui.ProductTokens

/**
 * The server's message for signed-in people (a new version, news): the first one for a version below the one named
 * that the person has not closed. Closing is kept on the device only.
 */
internal object HomeAnnouncements {
    fun pick(context: Context, announcements: List<Announcement>): Announcement? =
        choose(announcements, closed(context), BuildConfig.VERSION_CODE)

    /** The rule alone: a version below the named one (0: any), not closed. */
    fun choose(announcements: List<Announcement>, closed: Set<String>, versionCode: Int): Announcement? =
        announcements.firstOrNull { it.id !in closed && (it.belowVersionCode <= 0 || versionCode < it.belowVersionCode) }

    fun close(context: Context, id: String) {
        val kept = (closed(context) + id).toList().takeLast(KEPT)
        prefs(context).edit().putString(StorageKeys.Key.ANNOUNCEMENTS_CLOSED, kept.joinToString("\n")).apply()
    }

    private fun closed(context: Context): Set<String> =
        prefs(context).getString(StorageKeys.Key.ANNOUNCEMENTS_CLOSED, null).orEmpty().split('\n').filter(String::isNotEmpty).toSet()

    private fun prefs(context: Context) = GovorunPreferences.of(context.applicationContext, StorageKeys.PREFS_FILE)

    /** A handful of the last closed ids is enough: the server keeps at most five announcements at a time. */
    private const val KEPT = 20
}

/** One message at the top of the home screen: what, a short line, its button and a close. */
@Composable
internal fun HomeAnnouncementCard(announcement: Announcement, onOpen: (String) -> Unit, onClose: () -> Unit) {
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Row(Modifier.padding(start = 20.dp, top = 16.dp, end = 4.dp, bottom = if (announcement.url.isEmpty()) 16.dp else 0.dp),
            horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Icon(PhosphorIcons.Regular.ArrowCircleUp, contentDescription = null, tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(top = 2.dp).size(24.dp))
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(announcement.title, style = MaterialTheme.typography.titleSmall)
                if (announcement.text.isNotEmpty()) {
                    Text(announcement.text, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            IconButton(onClick = onClose, modifier = Modifier.size(36.dp)) {
                Icon(PhosphorIcons.Regular.X, contentDescription = "Закрыть", modifier = Modifier.size(18.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (announcement.url.isNotEmpty()) {
            TextButton(onClick = { onOpen(announcement.url) }, modifier = Modifier.padding(start = 50.dp, bottom = 4.dp).align(Alignment.Start)) {
                Text(announcement.action)
            }
        }
    }
}
