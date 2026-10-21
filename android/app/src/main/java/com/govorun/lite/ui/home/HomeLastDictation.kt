package com.govorun.lite.ui.home

import android.content.ClipData
import android.content.ClipboardManager
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Check
import com.adamglin.phosphoricons.regular.Copy
import com.adamglin.phosphoricons.regular.Microphone
import kotlinx.coroutines.delay
import com.product.ui.ProductTokens

/**
 * Shown only when «Помнить последнюю диктовку» is on and a dictation happened since launch.
 * Collapsed like «Статистика»; the copy icon copies the pasted text without expanding. When online
 * improvement changed it, «Улучшенный» and «Оригинал» each get the same icon, as on the desktop.
 */
@Composable
internal fun LastDictationCard(improved: String, original: String? = null) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    Card(
        modifier = Modifier
            .feedCard()
            .animateContentSize(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column {
            FeedSectionHeader(
                title = "Последняя диктовка",
                icon = PhosphorIcons.Regular.Microphone,
                expanded = expanded,
                onToggle = { expanded = !expanded },
            ) {
                CopyButton(improved, "Скопировать")
            }
            AnimatedVisibility(expanded) {
                Column {
                    CompactDivider()
                    if (original == null) {
                        Column(Modifier.padding(ProductTokens.Spacing.Section)) { DictationPreview(improved) }
                    } else {
                        DictationVersion("Улучшенный", improved)
                        CompactDivider()
                        DictationVersion("Оригинал", original)
                    }
                }
            }
        }
    }
}

@Composable
private fun DictationVersion(label: String, text: String) {
    // The copy icon lines up with the header's: the header row uses the narrower Lg side padding.
    Column(
        Modifier.padding(
            start = ProductTokens.Spacing.Section,
            end = ProductTokens.Spacing.Lg,
            bottom = ProductTokens.Spacing.Section,
        ),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                "$label · ${text.wordCount().wordsLabel()}",
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            CopyButton(text, "Скопировать: ${label.lowercase()}")
        }
        Column(Modifier.padding(end = ProductTokens.Spacing.Section - ProductTokens.Spacing.Lg)) { DictationPreview(text) }
    }
}

/** The icon button of the home cards; a check mark confirms the copy for a moment. */
@Composable
private fun CopyButton(text: String, description: String) {
    val context = LocalContext.current
    var copied by remember(text) { mutableStateOf(false) }
    LaunchedEffect(copied) {
        if (copied) { delay(1_500); copied = false }
    }
    IconButton(onClick = {
        context.getSystemService(ClipboardManager::class.java)
            .setPrimaryClip(ClipData.newPlainText("Последняя диктовка", text))
        copied = true
    }) {
        Icon(
            if (copied) PhosphorIcons.Regular.Check else PhosphorIcons.Regular.Copy,
            contentDescription = description,
            modifier = Modifier.size(ProductTokens.Size.TopBarActionIcon),
        )
    }
}

/** Six lines at most; a longer text says so, the copy is always whole. */
@Composable
private fun DictationPreview(text: String) {
    var cut by remember(text) { mutableStateOf(false) }
    Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm)) {
        Text(
            text,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = PREVIEW_LINES,
            overflow = TextOverflow.Ellipsis,
            onTextLayout = { cut = it.hasVisualOverflow },
        )
        if (cut) {
            Text(
                "Показано начало · копируется весь текст",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = ProductTokens.Alpha.MutedIcon),
            )
        }
    }
}

private const val PREVIEW_LINES = 6

private fun String.wordCount(): Long = split(Regex("\\s+")).count { it.isNotEmpty() }.toLong()
