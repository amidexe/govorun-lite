package com.govorun.lite.ui

import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.drawable.Drawable
import android.os.Bundle
import android.widget.ImageView
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.filter.AppFilterScreen
import com.govorun.lite.ui.filter.FilterApp
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class AppFilterActivity : ComponentActivity() {
    private var apps by mutableStateOf<List<FilterApp>>(emptyList())
    private var mode by mutableStateOf(Prefs.APP_FILTER_BLACKLIST)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        mode = Prefs.getAppFilterMode(this)
        val savedPackages = Prefs.getAppFilterPackages(this)
        setContent {
            GovorunTheme {
                AppFilterScreen(
                    apps = apps,
                    mode = mode,
                    onBack = ::finish,
                    onMode = ::changeMode,
                    onToggle = ::toggleApp,
                )
            }
        }
        lifecycleScope.launch {
            apps = withContext(Dispatchers.IO) { loadApps(savedPackages) }
        }
    }

    private fun changeMode(value: String) {
        mode = value
        save()
    }

    private fun toggleApp(packageName: String) {
        apps = apps.map {
            if (it.packageName == packageName) it.copy(checked = !it.checked) else it
        }.sortedWith(compareByDescending<FilterApp> { it.checked }.thenBy { it.label.lowercase() })
        save()
    }

    private fun save() {
        Prefs.setAppFilterMode(this, mode)
        Prefs.setAppFilterPackages(
            this,
            apps.filter { it.checked }.mapTo(mutableSetOf()) { it.packageName },
        )
    }

    private fun loadApps(saved: Set<String>): List<FilterApp> {
        val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        return packageManager.queryIntentActivities(launcher, 0)
            .map { it.activityInfo.packageName }
            .distinct()
            .filter { it != packageName }
            .map { packageName ->
                val info = runCatching {
                    packageManager.getApplicationInfo(packageName, 0)
                }.getOrNull()
                FilterApp(
                    label = info?.let(packageManager::getApplicationLabel)?.toString()
                        ?: packageName,
                    packageName = packageName,
                    icon = info?.let(packageManager::getApplicationIcon),
                    checked = packageName in saved,
                )
            }
            .sortedWith(compareByDescending<FilterApp> { it.checked }.thenBy { it.label.lowercase() })
    }
}
