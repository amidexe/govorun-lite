package com.govorun.lite.ui

import android.content.Intent
import android.net.Uri
import android.widget.Toast
import androidx.activity.ComponentActivity
import com.govorun.lite.account.GovorunAccountRepository
import java.util.UUID

internal fun ComponentActivity.openSubscriptionSupport(details: List<String>) {
    val subject = Uri.encode("Говорун — подписка")
    val body = Uri.encode(
        subscriptionSupportBody(GovorunAccountRepository.state.value.accountId, details),
    )
    val intent = Intent(
        Intent.ACTION_SENDTO,
        Uri.parse("mailto:$SUBSCRIPTION_SUPPORT_EMAIL?subject=$subject&body=$body"),
    )
    runCatching { startActivity(intent) }
        .onFailure {
            Toast.makeText(
                this,
                "Напишите на $SUBSCRIPTION_SUPPORT_EMAIL",
                Toast.LENGTH_LONG,
            ).show()
        }
}

internal fun subscriptionSupportBody(accountId: String, details: List<String> = emptyList()): String {
    val supportReference = accountId.takeIf {
        runCatching { UUID.fromString(it) }.isSuccess
    } ?: "не определён"
    return (listOf("Код аккаунта: $supportReference") + details).joinToString("\n") + "\n\n" +
        "Опишите вопрос об оплате, отмене или возврате:"
}
