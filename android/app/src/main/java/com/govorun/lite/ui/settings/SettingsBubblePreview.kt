package com.govorun.lite.ui.settings

import com.govorun.lite.overlay.BubblePlacement
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.overlay.BubbleThemeColors
import com.govorun.lite.ui.design.GovorunSpacing
import com.govorun.lite.util.Prefs
import com.product.ui.ProductTokens

@Composable
internal fun BubblePreview(state: SettingsUiState) {
    val scheme = MaterialTheme.colorScheme
    val context = LocalContext.current
    val bubbleColors = BubbleThemeColors(
        primary = scheme.primary.toArgb(),
        onPrimary = scheme.onPrimary.toArgb(),
        primaryContainer = scheme.primaryContainer.toArgb(),
        onPrimaryContainer = scheme.onPrimaryContainer.toArgb(),
        onSurface = scheme.onSurface.toArgb(),
        surfaceContainerHigh = scheme.surfaceContainerHigh.toArgb(),
        error = scheme.error.toArgb(),
    )
    val left = state.bubbleSide == Prefs.BUBBLE_SIDE_LEFT
    // The same window offset the floating bird gets, so the edge margin shows to scale.
    val offsetPx = remember(state.edgeMargin, state.edgeMarginCustomized, state.bubbleSize) {
        BubblePlacement.horizontalOffsetPx(context)
    }
    val offset = with(LocalDensity.current) { offsetPx.toDp() }
    val lineColor = scheme.outlineVariant.copy(alpha = 0.6f)
    val frameColor = scheme.outline.copy(alpha = 0.55f)
    val page = scheme.surface
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = ProductTokens.Spacing.Section, vertical = GovorunSpacing.medium)
            // A small phone: its screen holds a page of text and the bird, and the
            // frame marks both edges the side and edge margin are measured from.
            .drawWithContent {
                val frame = PREVIEW_FRAME.toPx()
                val screen = RoundRect(
                    left = frame, top = frame, right = size.width - frame, bottom = size.height - frame,
                    cornerRadius = CornerRadius(PREVIEW_SCREEN_RADIUS.toPx()),
                )
                drawRoundRect(
                    frameColor,
                    cornerRadius = CornerRadius(PREVIEW_SCREEN_RADIUS.toPx() + frame),
                )
                clipPath(Path().apply { addRoundRect(screen) }) {
                    drawRect(page)
                    val step = 14.dp.toPx()
                    var y = frame + step
                    var row = 0
                    while (y < size.height - frame) {
                        val end = size.width * if (row % 3 == 2) 0.55f else 0.9f
                        drawLine(lineColor, Offset(size.width * 0.1f, y), Offset(end, y), 3.dp.toPx(), StrokeCap.Round)
                        y += step
                        row++
                    }
                    this@drawWithContent.drawContent()
                }
                // Front camera, so the frame reads as a phone at a glance.
                drawCircle(frameColor, radius = 2.5.dp.toPx(), center = Offset(size.width / 2f, frame + 7.dp.toPx()))
            },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = PREVIEW_FRAME + GovorunSpacing.large),
        ) {
            listOf(
                BubbleView.Presentation.IDLE to "В покое",
                BubbleView.Presentation.CONFIRM_RECORDING to "Запись",
            ).forEach { (presentation, label) ->
                // Fits the largest bird's touch area (110 dp tall, drawn 67 dp), so changing
                // the size never moves the screen.
                Box(
                    modifier = Modifier.fillMaxWidth().height(PREVIEW_ROW_HEIGHT),
                    contentAlignment = if (left) Alignment.CenterStart else Alignment.CenterEnd,
                ) {
                    AndroidView(
                        // The view's own touch padding may be taller than the row; let it overflow.
                        modifier = Modifier
                            .wrapContentHeight(unbounded = true)
                            .offset(x = if (left) PREVIEW_FRAME + offset else -(PREVIEW_FRAME + offset)),
                        factory = { viewContext ->
                            BubbleView(viewContext).apply {
                                importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_NO
                            }
                        },
                        update = { bubble ->
                            bubble.setThemeColors(bubbleColors)
                            bubble.applySizeScaleFromPrefs()
                            bubble.setBirdColor(state.birdColor)
                            bubble.setIdleAlpha(state.bubbleAlpha)
                            bubble.setRecordingAlpha(state.recordingAlpha)
                            bubble.setPresentation(presentation, anchorOnLeft = left)
                        },
                    )
                    Text(
                        label,
                        style = MaterialTheme.typography.labelSmall,
                        color = scheme.onSurfaceVariant,
                        modifier = Modifier
                            .align(if (left) Alignment.CenterEnd else Alignment.CenterStart)
                            .padding(horizontal = PREVIEW_FRAME + GovorunSpacing.large)
                            .background(scheme.surface, MaterialTheme.shapes.small)
                            .padding(horizontal = GovorunSpacing.small),
                    )
                }
            }
        }
    }
}

private val PREVIEW_ROW_HEIGHT = 72.dp
private val PREVIEW_FRAME = 4.dp
private val PREVIEW_SCREEN_RADIUS = 22.dp
