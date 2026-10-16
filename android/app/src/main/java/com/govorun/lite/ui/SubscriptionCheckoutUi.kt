package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import com.govorun.lite.account.BillingPlan
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard

@Composable
internal fun SubscriptionPlanSelector(
    plans: List<BillingPlan>,
	selectedOfferId: String,
    trialDays: Int?,
    onSelect: (BillingPlan) -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth().selectableGroup(),
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
    ) {
        plans.forEach { plan ->
            PlanCard(
                plan = plan,
                active = false,
				selected = plan.offerId == selectedOfferId,
                selectable = true,
                trialDays = trialDays,
                limit = null,
                onSelect = { onSelect(plan) },
            )
        }
    }
}

@Composable
internal fun CheckoutActionSection(
    plan: BillingPlan,
    trialDays: Int?,
    enabled: Boolean,
    unavailableMessage: String? = null,
    onChoose: () -> Unit,
    actionTitle: String? = null,
    showDisclosure: Boolean = true,
) {
    val presentation = checkoutOfferPresentation(plan, trialDays)
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Section),
    ) {
        if (unavailableMessage != null) {
            Text(
                unavailableMessage,
                modifier = Modifier.fillMaxWidth().padding(horizontal = ProductTokens.Spacing.Screen),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        } else {
            Button(
                onClick = onChoose,
                enabled = enabled,
                modifier = Modifier
                    .productFeedCard()
                    .heightIn(min = ProductTokens.Size.TouchTarget),
                shape = RoundedCornerShape(ProductTokens.Radius.Card),
            ) {
                Text(
                    actionTitle ?: presentation.action,
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
            if (showDisclosure) Text(
                presentation.disclosure,
                modifier = Modifier.fillMaxWidth().padding(horizontal = ProductTokens.Spacing.Screen),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

internal data class CheckoutOfferPresentation(
    val disclosure: String,
    val action: String,
)

internal fun checkoutOfferPresentation(
    plan: BillingPlan,
    trialDays: Int?,
): CheckoutOfferPresentation {
    return CheckoutOfferPresentation(
        disclosure = if (trialDays != null) {
            "Первое списание — через $trialDays дней, затем — автопродление. Отменить подписку можно в любое время. Если отменить до первого списания, оплаты не будет."
        } else {
            "Отменить подписку можно в любое время."
        },
        action = if (trialDays != null) {
            "Попробовать бесплатно"
        } else {
            "Подключить"
        },
    )
}
