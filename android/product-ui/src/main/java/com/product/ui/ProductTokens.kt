package com.product.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

object ProductTokens {
    object Color {
        // DayPlan's palette: one teal accent for light and dark.
        val BrandAccent = Color(0xFF216E72)
        val BrandAccentDark = Color(0xFF216E72)
        val Warning = Color(0xFF946200)
        val WarningDark = Color(0xFFFFD166)
        val Success = Color(0xFF2E7D32)
        val SuccessDark = Color(0xFF81C784)
        val LightBackground = Color(0xFFFBFAF7)
        val LightSurface = Color(0xFFFFFFFF)
        val LightMutedSurface = Color(0xFFF3EFEC)
        val LightText = Color(0xFF16100C)
        val LightMutedText = Color(0xFF716A63)
        val DarkBackground = Color(0xFF181818)
        val DarkSurface = Color(0xFF242424)
        val DarkMutedSurface = Color(0xFF303030)
        val DarkText = Color(0xFFF7F7F7)
        val DarkMutedText = Color(0xFFD0D0D0)
    }

    object Spacing {
        val Xxs = 2.dp
        val Xs = 3.dp
        val Sm = 4.dp
        val Md = 6.dp
        val Lg = 8.dp
        val Xl = 10.dp
        val Xxl = 12.dp
        val Section = 14.dp
        val Screen = 18.dp
    }

    object Radius {
        val Xs = 6.dp
        val Sm = 8.dp
        val Md = 12.dp
        val Lg = 16.dp
        val Card = Lg
        val Xl = 24.dp
    }

    object Size {
        val TouchTarget = 48.dp
        val Row = 48.dp
        val TopBarNavigationIcon = 20.dp
        val TopBarActionIcon = 18.dp
        val SlotIcon = 14.dp
        val ActiveIcon = 18.dp
        val CompactSwitchWidth = 34.dp
        val CompactSwitchHeight = 20.dp
    }

    object Alpha {
        const val MutedIcon = 0.72f
        const val Divider = 0.30f
        const val SelectedSurface = 0.64f
    }
}
