package com.govorun.lite.ui

import android.content.Intent
import android.graphics.drawable.Drawable
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.ApplicationStyleProfile
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.StylizationStylePreferences
import com.govorun.lite.stylization.titleRes
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.filter.AppRuleItem
import com.govorun.lite.ui.filter.AppRulesScreen
import com.product.ui.ProductDivider
import com.product.ui.ProductTokens
import com.product.ui.SettingsSwitchRow

class StylizationAppRulesActivity : ComponentActivity() {
    private var rules by mutableStateOf<Map<String, ApplicationStyleProfile>>(emptyMap())
    private var apps by mutableStateOf<List<AppEntry>>(emptyList())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        rules = StylizationStylePreferences.appProfiles(this)
        apps = loadApps()
        setContent {
            GovorunTheme {
                var selected by remember { mutableStateOf<AppEntry?>(null) }
                AppRulesScreen(
                    apps = apps.map { app ->
                        AppRuleItem(
                            app.name,
                            app.packageName,
                            app.icon,
                            profileLabel(rules[app.packageName]),
                        )
                    },
                    onBack = ::finish,
                    onSelect = { item -> selected = apps.first { it.packageName == item.packageName } },
                )
                selected?.let { app ->
                    ProfileSheet(app, rules[app.packageName], { selected = null }) { profile ->
                        setProfile(app, profile)
                        selected = null
                    }
                }
            }
        }
    }

    private fun setProfile(app: AppEntry, profile: ApplicationStyleProfile?) {
        StylizationStylePreferences.setAppProfile(this, app.packageName, profile)
        rules = StylizationStylePreferences.appProfiles(this)
        apps = apps.sortedWith(
            compareByDescending<AppEntry> { rules.containsKey(it.packageName) }
                .thenBy { it.name.lowercase() },
        )
    }

    private fun loadApps(): List<AppEntry> {
        val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        return packageManager.queryIntentActivities(launcher, 0)
            .distinctBy { it.activityInfo.packageName }
            .filter { it.activityInfo.packageName != packageName }
            .map {
                AppEntry(
                    it.loadLabel(packageManager).toString(),
                    it.activityInfo.packageName,
                    it.loadIcon(packageManager),
                )
            }
            .sortedWith(
                compareByDescending<AppEntry> { rules.containsKey(it.packageName) }
                    .thenBy { it.name.lowercase() },
            )
    }

    private data class AppEntry(
        val name: String,
        val packageName: String,
        val icon: Drawable?,
    )

    @OptIn(ExperimentalMaterial3Api::class)
    @Composable
    private fun ProfileSheet(
        app: AppEntry,
        current: ApplicationStyleProfile?,
        dismiss: () -> Unit,
        save: (ApplicationStyleProfile?) -> Unit,
    ) {
        val choices = remember { styleChoices() }
        var styleValue by rememberSaveable(app.packageName) {
            mutableStateOf(current?.styleValue)
        }
        var customOptions by rememberSaveable(app.packageName) {
            mutableStateOf(current?.emojiEnabled != null || current?.emojiMode != null ||
                current?.reducedPunctuationEnabled != null)
        }
        val inherited = inheritedOptions(styleValue)
        var emojiMode by rememberSaveable(app.packageName) {
            mutableStateOf(current?.emojiMode ?: current?.emojiEnabled?.let {
                EmojiMode.fromApiId(null, it)
            } ?: inherited.resolvedEmojiMode)
        }
        var reducedPunctuation by rememberSaveable(app.packageName) {
            mutableStateOf(current?.reducedPunctuationEnabled ?:
                (inherited.punctuation == PunctuationMode.LESS))
        }
        ModalBottomSheet(onDismissRequest = dismiss) {
            Column(
                modifier = Modifier.padding(
                    start = ProductTokens.Spacing.Section,
                    end = ProductTokens.Spacing.Section,
                    bottom = ProductTokens.Spacing.Xl,
                ),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                Text(app.name, style = MaterialTheme.typography.titleLarge)
                Text(
                    "Стиль текста",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
                    verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxs),
                ) {
                    choices.forEach { choice ->
                        FilterChip(
                            selected = styleValue == choice.value,
                            onClick = { styleValue = choice.value },
                            label = { Text(choice.label) },
                        )
                    }
                }
                ProductDivider()
                SettingsSwitchRow(
                    title = "Свои параметры для приложения",
                    summary = "Иначе используются общие настройки оформления текста.",
                    checked = customOptions,
                    onCheckedChange = { customOptions = it },
                )
                if (customOptions) {
                    Text("Эмодзи", style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxs),
                    ) {
                        EmojiMode.entries.forEach { mode ->
                            FilterChip(
                                selected = emojiMode == mode,
                                onClick = { emojiMode = mode },
                                label = { Text(emojiModeLabel(mode)) },
                            )
                        }
                    }
                    SettingsSwitchRow(
                        title = "Меньше пунктуации",
                        summary = "Применяется только в ${app.name}.",
                        checked = reducedPunctuation,
                        onCheckedChange = { reducedPunctuation = it },
                    )
                }
                Spacer(Modifier.height(4.dp))
                Button(
                    modifier = Modifier.fillMaxWidth(),
                    onClick = {
                        save(ApplicationStyleProfile(
                            styleValue = styleValue,
                            emojiEnabled = emojiMode.enabled.takeIf { customOptions },
                            emojiMode = emojiMode.takeIf { customOptions },
                            reducedPunctuationEnabled = reducedPunctuation
                                .takeIf { customOptions },
                        ).takeUnless(ApplicationStyleProfile::isInherited))
                    },
                ) { Text("Готово") }
                if (current != null) {
                    TextButton(
                        modifier = Modifier.fillMaxWidth(),
                        onClick = { save(null) },
                    ) { Text("Удалить правило") }
                }
            }
        }
    }

    private data class StyleChoice(val value: String?, val label: String)

    private fun styleChoices(): List<StyleChoice> = buildList {
        add(StyleChoice(null, "Как в общих настройках"))
        add(StyleChoice("off", "Без стилизации"))
        TextStyle.entries.filterNot { it == TextStyle.CUSTOM }.forEach {
            add(StyleChoice(it.apiId, getString(it.titleRes)))
        }
        CustomStylePresets.list(this@StylizationAppRulesActivity).forEach {
            add(StyleChoice("custom:${it.id}", it.name))
        }
    }

    private fun inheritedOptions(styleValue: String?) = when {
        styleValue == null || styleValue == "off" ->
            StylizationStylePreferences.optionsForSelected(this)
        else -> StylizationStylePreferences.options(this, styleValue)
    }

    private fun profileLabel(profile: ApplicationStyleProfile?): String {
        if (profile == null) return "Как в общих настройках"
        val style = when (val value = profile.styleValue) {
            null -> "Общий стиль"
            "off" -> "Без стилизации"
            else -> if (value.startsWith("custom:")) {
                CustomStylePresets.list(this)
                    .firstOrNull { it.id == value.removePrefix("custom:") }
                    ?.name ?: "Свой стиль"
            } else TextStyle.fromApiId(value).let { getString(it.titleRes) }
        }
        val custom = profile.emojiEnabled != null || profile.emojiMode != null ||
            profile.reducedPunctuationEnabled != null
        return if (custom) "$style · свои параметры" else style
    }

    private fun emojiModeLabel(mode: EmojiMode): String = when (mode) {
        EmojiMode.OFF -> "Выкл."
        EmojiMode.RESTRAINED -> "Сдержанно"
        EmojiMode.LIVELY -> "Живо"
        EmojiMode.BRIGHT -> "Ярко"
    }
}
