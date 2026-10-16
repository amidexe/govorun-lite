package com.govorun.lite.ui.design

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.core.view.WindowCompat
import com.govorun.lite.R
import com.govorun.lite.util.UiAppearancePrefs
import com.govorun.lite.model.settings.UiThemeMode
import com.product.ui.ProductTheme
import com.product.ui.ProductTokens
import com.product.ui.productTypography

private val Geist = FontFamily(
    Font(R.font.geist_variable, FontWeight.Normal),
    Font(R.font.geist_variable, FontWeight.Medium),
    Font(R.font.geist_variable, FontWeight.SemiBold),
    Font(R.font.geist_variable, FontWeight.Bold),
)

private val GovorunTypography = productTypography(Geist)

@Composable
fun GovorunTheme(
    content: @Composable () -> Unit,
) {
    val appearance by UiAppearancePrefs.appearance
    val darkTheme = when (appearance.themeMode) {
        UiThemeMode.LIGHT -> false
        UiThemeMode.DARK -> true
        UiThemeMode.SYSTEM -> isSystemInDarkTheme()
    }
    GovorunTheme(darkTheme, appearance.dynamicColor, content)
}

/** Explicit appearance for isolated previews; the normal entry point uses saved preferences. */
@Composable
internal fun GovorunTheme(darkTheme: Boolean, dynamicColor: Boolean, content: @Composable () -> Unit) {
    val view = LocalView.current
    SideEffect {
        view.context.findActivity()?.window?.let { window ->
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !darkTheme
                isAppearanceLightNavigationBars = !darkTheme
            }
        }
    }
    ProductTheme(
        darkTheme = darkTheme,
        dynamicColor = dynamicColor,
        typography = GovorunTypography,
        content = content,
    )
}

private tailrec fun Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}

object GovorunSpacing {
    val xSmall = ProductTokens.Spacing.Xxs
    val small = ProductTokens.Spacing.Sm
    val medium = ProductTokens.Spacing.Md
    val large = ProductTokens.Spacing.Lg
    val xLarge = ProductTokens.Spacing.Section
    val xxLarge = ProductTokens.Spacing.Screen
}
