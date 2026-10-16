package com.product.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.PlatformTextStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontSynthesis
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp

private val LightColors = lightColorScheme(
    primary = ProductTokens.Color.BrandAccent,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFEADDFF),
    onPrimaryContainer = Color(0xFF21005D),
    secondary = ProductTokens.Color.LightText,
    onSecondary = ProductTokens.Color.LightSurface,
    secondaryContainer = Color(0xFFEDD5CE),
    onSecondaryContainer = Color(0xFF211B16),
    background = ProductTokens.Color.LightBackground,
    onBackground = ProductTokens.Color.LightText,
    surface = ProductTokens.Color.LightSurface,
    onSurface = ProductTokens.Color.LightText,
    surfaceVariant = ProductTokens.Color.LightMutedSurface,
    onSurfaceVariant = ProductTokens.Color.LightMutedText,
    surfaceContainerLowest = ProductTokens.Color.LightSurface,
    surfaceContainerLow = ProductTokens.Color.LightMutedSurface,
    surfaceContainer = Color(0xFFF2EEE9),
    surfaceContainerHigh = Color(0xFFEBE7E2),
    surfaceContainerHighest = Color(0xFFE0DAD4),
    outline = Color(0xFF9E9790),
    outlineVariant = Color(0xFFB5AFA8),
    error = Color(0xFFE34A55),
    errorContainer = Color(0xFFFFE2E2),
    onErrorContainer = Color(0xFF4C0608),
)

private val DarkColors = darkColorScheme(
    primary = ProductTokens.Color.BrandAccentDark,
    onPrimary = Color(0xFF381E72),
    primaryContainer = Color(0xFF4F378B),
    onPrimaryContainer = Color(0xFFEADDFF),
    secondary = ProductTokens.Color.DarkText,
    onSecondary = ProductTokens.Color.DarkBackground,
    background = ProductTokens.Color.DarkBackground,
    onBackground = ProductTokens.Color.DarkText,
    surface = ProductTokens.Color.DarkSurface,
    onSurface = ProductTokens.Color.DarkText,
    surfaceContainerLowest = ProductTokens.Color.DarkBackground,
    surfaceContainerLow = ProductTokens.Color.DarkSurface,
    surfaceContainer = ProductTokens.Color.DarkSurface,
    surfaceContainerHigh = ProductTokens.Color.DarkMutedSurface,
    surfaceContainerHighest = Color(0xFF383838),
    onSurfaceVariant = ProductTokens.Color.DarkMutedText,
    outline = Color(0xFF707070),
    outlineVariant = Color(0xFF454545),
    error = Color(0xFFFF6A70),
    errorContainer = Color(0xFF651114),
    onErrorContainer = Color(0xFFFFE2E2),
)

private val ProductShapes = Shapes(
    extraSmall = RoundedCornerShape(ProductTokens.Radius.Xs),
    small = RoundedCornerShape(ProductTokens.Radius.Sm),
    medium = RoundedCornerShape(ProductTokens.Radius.Md),
    large = RoundedCornerShape(ProductTokens.Radius.Card),
    extraLarge = RoundedCornerShape(ProductTokens.Radius.Xl),
)

@Composable
fun ProductTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    typography: Typography,
    content: @Composable () -> Unit,
) {
    val usesDynamic = dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
    val colors = when {
        usesDynamic && darkTheme -> readableDynamic(dynamicDarkColorScheme(LocalContext.current), true)
        usesDynamic -> readableDynamic(dynamicLightColorScheme(LocalContext.current), false)
        darkTheme -> DarkColors
        else -> LightColors
    }
    MaterialTheme(colors, ProductShapes, typography, content)
}

/** DayPlan font metrics, reduced to six shared roles across all app surfaces. */
fun productTypography(fontFamily: FontFamily): Typography {
    val screenTitle = productTextStyle(fontFamily, FontWeight.SemiBold, 17.sp, 22.sp)
    val title = productTextStyle(fontFamily, FontWeight.SemiBold, 15.sp, 20.sp)
    val body = productTextStyle(fontFamily, FontWeight.Normal, 14.sp, 20.sp)
    val caption = productTextStyle(fontFamily, FontWeight.Medium, 11.sp, 14.sp)
    val action = productTextStyle(fontFamily, FontWeight.Medium, 13.sp, 18.sp)
    val metric = productTextStyle(fontFamily, FontWeight.SemiBold, 22.sp, 28.sp)
    // Material controls and app Text calls resolve through the same small role set.
    return Typography(
        displayLarge = metric,
        displayMedium = metric,
        displaySmall = metric,
        headlineLarge = metric,
        headlineMedium = metric,
        headlineSmall = title,
        titleLarge = screenTitle,
        titleMedium = title,
        titleSmall = title,
        bodyLarge = body,
        bodyMedium = body,
        bodySmall = caption,
        labelLarge = action,
        labelMedium = caption,
        labelSmall = caption,
    )
}

private fun productTextStyle(
    family: FontFamily,
    weight: FontWeight,
    size: TextUnit,
    height: TextUnit,
) = TextStyle(
    fontFamily = family,
    fontWeight = weight,
    fontSynthesis = FontSynthesis.None,
    fontSize = size,
    lineHeight = height,
    fontFeatureSettings = "\"cv11\", \"ss01\", \"ss03\"",
    platformStyle = PlatformTextStyle(includeFontPadding = false),
    lineHeightStyle = LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.None),
)

private fun readableDynamic(dynamic: ColorScheme, dark: Boolean): ColorScheme =
    if (dark) {
        dynamic.copy(
            error = Color(0xFFFF6A70),
            errorContainer = Color(0xFF651114),
            onErrorContainer = Color(0xFFFFE2E2),
            onSurfaceVariant = lerp(dynamic.surface, dynamic.onSurface, 0.88f),
            outline = lerp(dynamic.surface, dynamic.outline, 0.78f),
            outlineVariant = lerp(dynamic.surface, dynamic.outlineVariant, 0.82f),
            surfaceContainerLow = dynamic.surfaceContainer,
            surfaceContainer = dynamic.surfaceContainerHigh,
            surfaceContainerHigh = dynamic.surfaceContainerHighest,
            surfaceContainerLowest = Color.Black,
        )
    } else {
        dynamic.copy(
            error = Color(0xFFE34A55),
            errorContainer = Color(0xFFFFE2E2),
            onErrorContainer = Color(0xFF4C0608),
            onSurfaceVariant = lerp(dynamic.surface, dynamic.onSurface, 0.72f),
            outline = lerp(dynamic.surface, dynamic.outline, 0.66f),
            outlineVariant = lerp(dynamic.surface, dynamic.outlineVariant, 0.72f),
            background = dynamic.surfaceContainerHigh,
            surfaceContainerLowest = dynamic.surface,
            surfaceContainerLow = dynamic.surface,
            surfaceContainer = dynamic.surfaceContainerLow,
            surfaceContainerHigh = dynamic.surfaceContainer,
        )
    }
