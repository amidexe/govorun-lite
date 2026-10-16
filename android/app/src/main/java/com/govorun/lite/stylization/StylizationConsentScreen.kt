package com.govorun.lite.stylization

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.model.stylization.StyleOptions
import java.util.WeakHashMap
import kotlinx.coroutines.Job
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch

internal data class ConsentStyle(val id: String, val title: String, val options: StyleOptions = StyleOptions())

private class ConsentSession(
    val styles: List<ConsentStyle>, val selected: String, val changesInputMode: Boolean, val afterSignIn: Boolean,
    val documents: () -> Unit, val accept: suspend (String, StyleOptions) -> Boolean,
    val result: (Boolean) -> Unit,
) {
    var job: Job? = null
    var delivered = false
    val account = com.govorun.lite.account.GovorunAccountRepository.state.value.takeIf { it.signedIn }
}
private class ConsentHost { var session by mutableStateOf<ConsentSession?>(null) }
private val hosts = WeakHashMap<Activity, ConsentHost>()
private fun Context.activity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.takeUnless { it === this }?.activity()
    else -> null
}
private fun host(activity: Activity): ConsentHost = hosts.getOrPut(activity) {
    ConsentHost().also { holder ->
        (activity as? LifecycleOwner)?.lifecycle?.addObserver(object : DefaultLifecycleObserver {
            override fun onDestroy(owner: LifecycleOwner) {
                val previous = holder.session
                holder.session = null
                previous?.job?.cancel()
                if (previous != null && !previous.delivered) previous.result(false)
                hosts.remove(activity)
                owner.lifecycle.removeObserver(this)
            }
        })
    }
}
private fun finish(holder: ConsentHost, session: ConsentSession, accepted: Boolean) {
    if (holder.session !== session) return
    holder.session = null
    if (!session.delivered) { session.delivered = true; session.result(accepted) }
}

internal fun showStylizationConsentScreen(
    activity: Activity, styles: List<ConsentStyle>, selectedStyle: String,
    changesInputMode: Boolean, onOpenTerms: () -> Unit,
    accept: suspend (String, StyleOptions) -> Boolean, onResult: (Boolean) -> Unit,
    afterSignIn: Boolean = false,
) {
    if (activity !is LifecycleOwner) return onResult(false)
    val holder = host(activity)
    if (holder.session != null) return onResult(false)
    holder.session = ConsentSession(styles, selectedStyle, changesInputMode, afterSignIn, onOpenTerms, accept, onResult)
}

@Composable
internal fun hasInlineStylizationConsent(): Boolean = LocalContext.current.activity()?.let { host(it).session != null } == true

/** Replaces the initiating card; no preferences change before explicit acceptance. */
@Composable
internal fun InlineStylizationConsentCard() {
    val activity = LocalContext.current.activity() ?: return
    val owner = activity as? LifecycleOwner ?: return
    val holder = host(activity)
    val session = holder.session ?: return
    val account by com.govorun.lite.account.GovorunAccountRepository.state.collectAsState()
    LaunchedEffect(account.accountId, account.sessionGeneration, account.signedIn, account.secureStorageProblem) {
        session.account?.let { original ->
            if (!com.govorun.lite.account.sameActivationAccount(original, account)) {
                session.job?.cancel()
                finish(holder, session, false)
            }
        }
    }
    key(session) {
        var busy by remember { mutableStateOf(false) }
        var error by remember { mutableStateOf(false) }
        StyleActivationCard(session.changesInputMode, session.afterSignIn,
            busy, error, session.documents,
            onCancel = { if (!busy) finish(holder, session, false) },
            needsSubscription = com.govorun.lite.account.styleSetupNeedsSubscription(account),
            onPlans = {
                finish(holder, session, false)
                activity.startActivity(android.content.Intent(activity, com.govorun.lite.ui.SubscriptionActivity::class.java))
            },
            onAccept = {
                busy = true; error = false
                session.job = owner.lifecycleScope.launch {
                    val options = session.styles.first { it.id == session.selected }.options
                    val accepted = runCatching { session.accept(session.selected, options) }.getOrDefault(false)
                    kotlinx.coroutines.currentCoroutineContext().ensureActive()
                    if (holder.session === session) {
                        busy = false
                        if (accepted) {
                            finish(holder, session, true)
                        } else error = true
                    }
                }
            })
    }
}
