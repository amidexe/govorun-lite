package com.govorun.lite.account

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.ui.AccountActivity
import com.govorun.lite.ui.MainActivity
import com.govorun.lite.ui.SubscriptionActivity
import com.govorun.lite.ui.design.GovorunTheme
import kotlinx.coroutines.launch

class AccountCallbackActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            GovorunTheme {
                Column(
                    modifier = Modifier.fillMaxSize(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    CircularProgressIndicator()
                    Text("Завершаем вход…", style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
        val callback = intent?.data
        lifecycleScope.launch {
            val signedIn = callback != null &&
                GovorunAccountRepository.completeAuthCallback(this@AccountCallbackActivity, callback)
            if (signedIn) {
                finishAccountLogin(this@AccountCallbackActivity)
            } else {
                startActivity(Intent(this@AccountCallbackActivity, AccountActivity::class.java)
                    .putExtra(AccountActivity.EXTRA_RESUME_LOGIN,
                        GovorunAccountRepository.state.value.deviceLimitReached))
                finish()
            }
        }
    }

}
