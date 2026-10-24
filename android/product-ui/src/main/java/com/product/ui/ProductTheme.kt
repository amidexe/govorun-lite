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

// The default palette is Android 16's Monochrome wallpaper style as the system builds it, read on the Pixel 10
// (light and dark). It goes through readableDynamic like any wallpaper palette, so without Dynamic Colors the app
// looks exactly as it does with them on a monochrome phone.
private val MonochromeLight = lightColorScheme(
    primary = Color(0xFF5F5F5F),
    onPrimary = Color(0xFFF9F9F9),
    primaryContainer = Color(0xFFEBEBEB),
    onPrimaryContainer = Color(0xFF575757),
    inversePrimary = Color(0xFFFFFFFF),
    primaryFixed = Color(0xFFEBEBEB),
    primaryFixedDim = Color(0xFFDDDDDD),
    onPrimaryFixed = Color(0xFF444444),
    onPrimaryFixedVariant = Color(0xFF616161),
    secondary = Color(0xFF5F5F5F),
    onSecondary = Color(0xFFF9F9F9),
    secondaryContainer = Color(0xFFE2E2E2),
    onSecondaryContainer = Color(0xFF525252),
    tertiary = Color(0xFF5F5F5F),
    tertiaryContainer = Color(0xFFFFFFFF),
    background = Color(0xFFF9F9F9),
    onBackground = Color(0xFF323232),
    surface = Color(0xFFF9F9F9),
    onSurface = Color(0xFF323232),
    surfaceVariant = Color(0xFFE2E2E2),
    onSurfaceVariant = Color(0xFF5F5F5F),
    surfaceDim = Color(0xFFDADADA),
    surfaceBright = Color(0xFFF9F9F9),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFF3F3F3),
    surfaceContainer = Color(0xFFEEEEEE),
    surfaceContainerHigh = Color(0xFFE8E8E8),
    surfaceContainerHighest = Color(0xFFE2E2E2),
    outline = Color(0xFF7B7B7B),
    outlineVariant = Color(0xFFB2B2B2),
    inverseSurface = Color(0xFF0E0E0E),
    inverseOnSurface = Color(0xFF9D9D9D),
    error = Color(0xFFBB1B1B),
    onError = Color(0xFFFFF7F6),
    errorContainer = Color(0xFFFE4E44),
    onErrorContainer = Color(0xFF570003),
)

private val MonochromeDark = darkColorScheme(
    primary = Color(0xFFF9F9F9),
    onPrimary = Color(0xFF5F5F5F),
    // The one change to the system values: its mid grey #A0A0A0 container made the quiet lines of the improvement
    // card unreadable, so the container takes the dark grey of the secondary one with the light text.
    primaryContainer = Color(0xFF3B3B3B),
    onPrimaryContainer = Color(0xFFE5E5E5),
    inversePrimary = Color(0xFF5F5F5F),
    primaryFixed = Color(0xFFEBEBEB),
    primaryFixedDim = Color(0xFFDDDDDD),
    onPrimaryFixed = Color(0xFF444444),
    onPrimaryFixedVariant = Color(0xFF616161),
    secondary = Color(0xFFC6C6C6),
    onSecondary = Color(0xFF404040),
    secondaryContainer = Color(0xFF3B3B3B),
    onSecondaryContainer = Color(0xFFBFBFBF),
    tertiary = Color(0xFFF9F9F9),
    tertiaryContainer = Color(0xFFEBEBEB),
    background = Color(0xFF0E0E0E),
    onBackground = Color(0xFFE5E5E5),
    surface = Color(0xFF0E0E0E),
    onSurface = Color(0xFFE5E5E5),
    surfaceVariant = Color(0xFF262626),
    onSurfaceVariant = Color(0xFFABABAB),
    surfaceDim = Color(0xFF0E0E0E),
    surfaceBright = Color(0xFF2C2C2C),
    surfaceContainerLowest = Color(0xFF000000),
    surfaceContainerLow = Color(0xFF131313),
    surfaceContainer = Color(0xFF191919),
    surfaceContainerHigh = Color(0xFF1F1F1F),
    surfaceContainerHighest = Color(0xFF262626),
    outline = Color(0xFF757575),
    outlineVariant = Color(0xFF484848),
    inverseSurface = Color(0xFFF9F9F9),
    inverseOnSurface = Color(0xFF555555),
    error = Color(0xFFFF7164),
    onError = Color(0xFF4A0002),
    errorContainer = Color(0xFFAC0C12),
    onErrorContainer = Color(0xFFFFB8B0),
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
        darkTheme -> readableDynamic(MonochromeDark, true)
        else -> readableDynamic(MonochromeLight, false)
    }
    MaterialTheme(colors, ProductShapes, typography, content)
}

/** DayPlan's type scale: Material 3 roles in Geist, captions at 12–13 sp so notes and summaries stay readable. */
fun productTypography(fontFamily: FontFamily): Typography {
    fun style(weight: FontWeight, size: Int, height: Int) = productTextStyle(fontFamily, weight, size.sp, height.sp)
    return Typography(
        displayLarge = style(FontWeight.SemiBold, 30, 36),
        displayMedium = style(FontWeight.SemiBold, 26, 32),
        displaySmall = style(FontWeight.SemiBold, 22, 28),
        headlineLarge = style(FontWeight.SemiBold, 22, 28),
        headlineMedium = style(FontWeight.SemiBold, 20, 26),
        headlineSmall = style(FontWeight.SemiBold, 18, 24),
        titleLarge = style(FontWeight.SemiBold, 17, 22),
        titleMedium = style(FontWeight.SemiBold, 15, 20),
        titleSmall = style(FontWeight.SemiBold, 14, 18),
        bodyLarge = style(FontWeight.Normal, 15, 21),
        bodyMedium = style(FontWeight.Normal, 14, 20),
        bodySmall = style(FontWeight.Normal, 13, 18),
        labelLarge = style(FontWeight.Medium, 13, 18),
        labelMedium = style(FontWeight.Medium, 12, 16),
        labelSmall = style(FontWeight.Medium, 11, 14),
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
            // Material 3 red and a darker quiet text: #E34A55 was 3.7:1 on the cards and the quiet text 4.4:1 on the
            // grey background, now 6.1:1 and 5.0:1.
            error = Color(0xFFBA1A1A),
            errorContainer = Color(0xFFFFE2E2),
            onErrorContainer = Color(0xFF4C0608),
            onSurfaceVariant = lerp(dynamic.surface, dynamic.onSurface, 0.76f),
            outline = lerp(dynamic.surface, dynamic.outline, 0.66f),
            outlineVariant = lerp(dynamic.surface, dynamic.outlineVariant, 0.72f),
            background = dynamic.surfaceContainerHigh,
            surfaceContainerLowest = dynamic.surface,
            surfaceContainerLow = dynamic.surface,
            surfaceContainer = dynamic.surfaceContainerLow,
            surfaceContainerHigh = dynamic.surfaceContainer,
        )
    }
