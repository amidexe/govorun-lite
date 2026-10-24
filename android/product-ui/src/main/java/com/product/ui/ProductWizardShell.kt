package com.product.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp

/** Echo's first-run frame: step marks on top, the step, and a full-width «Далее» with a plain «Назад». */
@Composable
fun ProductWizardShell(
    step: Int,
    totalSteps: Int,
    canContinue: Boolean,
    continueLabel: String,
    onBack: (() -> Unit)?,
    onContinue: () -> Unit,
    content: @Composable () -> Unit,
) {
    // The screen's own Surface, as a Scaffold gives the others: titles take the text colour of the background.
    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(Modifier.fillMaxSize().statusBarsPadding().navigationBarsPadding()) {
            StepDots(current = step - 1, total = totalSteps)
            Box(
                modifier = Modifier.fillMaxWidth().weight(1f),
                contentAlignment = Alignment.TopCenter,
            ) {
                content()
            }
            Row(
                modifier = Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                if (onBack != null) {
                    TextButton(onClick = onBack, modifier = Modifier.height(52.dp)) { Text("Назад") }
                }
                Button(
                    onClick = onContinue,
                    enabled = canContinue,
                    modifier = Modifier.weight(1f).height(52.dp),
                ) {
                    Text(continueLabel, style = MaterialTheme.typography.titleMedium)
                }
            }
        }
    }
}

/** Echo's step marks: the current step is a longer pill, passed ones keep the accent, so what is left reads at a glance. */
@Composable
private fun StepDots(
    current: Int,
    total: Int,
) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = 20.dp, bottom = 8.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        repeat(total) { i ->
            Box(
                Modifier
                    .padding(horizontal = 3.dp)
                    .height(6.dp)
                    .width(if (i == current) 24.dp else 6.dp)
                    .clip(CircleShape)
                    .background(if (i <= current) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant),
            )
        }
    }
}
