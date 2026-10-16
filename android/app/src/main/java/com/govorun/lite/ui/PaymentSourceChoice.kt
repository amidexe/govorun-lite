package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp

@Composable
internal fun PaymentSourceChoice(selected: String, available: List<String>, working: Boolean, onSelect: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Способ оплаты", style = MaterialTheme.typography.titleSmall)
        Column(Modifier.selectableGroup()) {
            listOf("sbp", "card").filter { it in available }.forEach { kind ->
                val chosen = selected == kind
                Row(
                    Modifier.fillMaxWidth().selectable(chosen, enabled = !working, role = Role.RadioButton,
                        onClick = { if (!chosen) onSelect(kind) }).padding(vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    PaymentSourceIcon(kind)
                    Text(if (kind == "sbp") "Система быстрых платежей" else "Банковская карта",
                        modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                    RadioButton(selected = chosen, enabled = !working, onClick = null)
                }
            }
        }
    }
}
