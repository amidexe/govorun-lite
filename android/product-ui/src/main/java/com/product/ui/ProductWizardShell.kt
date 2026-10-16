package com.product.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun ProductWizardShell(
    title: String,
    step: Int,
    totalSteps: Int,
    canContinue: Boolean,
    continueLabel: String,
    onBack: (() -> Unit)?,
    onContinue: () -> Unit,
    content: @Composable () -> Unit,
) {
    ProductScaffold(title = title) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            LinearProgressIndicator(
                progress = { step.toFloat() / totalSteps.coerceAtLeast(1) },
                modifier = Modifier.fillMaxWidth(),
            )
            Box(
                modifier = Modifier.fillMaxWidth().weight(1f),
                contentAlignment = Alignment.TopCenter,
            ) {
                content()
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(
                        start = 24.dp,
                        end = 24.dp,
                        top = 12.dp,
                        bottom = 16.dp,
                    ),
                horizontalArrangement = Arrangement.Start,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (onBack != null) {
                    OutlinedButton(
                        onClick = onBack,
                        shape = MaterialTheme.shapes.extraLarge,
                    ) {
                        Text("Назад")
                    }
                }
                Spacer(Modifier.weight(1f))
                Button(
                    onClick = onContinue,
                    enabled = canContinue,
                    modifier = Modifier.defaultMinSize(minWidth = 140.dp),
                    shape = MaterialTheme.shapes.extraLarge,
                ) {
                    Text(continueLabel)
                }
            }
        }
    }
}
