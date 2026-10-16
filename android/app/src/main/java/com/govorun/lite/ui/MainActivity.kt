package com.govorun.lite.ui

import android.Manifest
import android.content.ContentResolver
import android.content.Intent
import android.content.pm.PackageManager
import android.database.ContentObserver
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.govorun.lite.account.BillingRepository
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.notifications.NotificationConsentRequest
import com.govorun.lite.notifications.NotificationInvitation
import com.govorun.lite.notifications.NotificationRepository
import com.govorun.lite.settings.AndroidStylizationSettings
import com.govorun.lite.stylization.AndroidCustomStyles
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.SelfHostedStylePresets
import com.govorun.lite.stylization.StylizationConsent
import com.govorun.lite.stylization.StylizationServerMode
import com.govorun.lite.stylization.StylizationServerPreferences
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.home.HomeEvent
import com.govorun.lite.ui.home.HomeScreen
import com.govorun.lite.ui.home.HomeSnapshotReader
import com.govorun.lite.ui.home.HomeUiState
import com.govorun.lite.util.AccessibilityHelper
import com.govorun.lite.util.HomePreferences
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.launch
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.withContext

/**
 * Govorun 2.0 Compose shell.
 *
 * The component name intentionally stays unchanged so a signed 1.x install can
 * update in place. Android side effects remain at this activity boundary; the
 * composable receives immutable state and emits explicit events.
 */
class MainActivity : ComponentActivity() {

    private lateinit var snapshotReader: HomeSnapshotReader
    private var uiState by mutableStateOf(HomeUiState.loading())
    private var showJustFinished = false
    private var refreshJob: Job? = null
    private var refreshRequested = false
    private var continueStylization = false
    private var styleJustEnabled by mutableStateOf(false)
    private var pendingNotificationConsent: NotificationConsentRequest? = null
    private val notificationPermission = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        pendingNotificationConsent?.let {
            if (granted) NotificationRepository.allow(it) else NotificationRepository.decline(it)
        }
        pendingNotificationConsent = null
        com.govorun.lite.stylization.OfficialServiceAvailability.get(this).requestRecovery()
        NotificationRepository.refresh()
    }
    private val notificationSettings = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) {
        pendingNotificationConsent?.let { request ->
            NotificationRepository.refresh()
            if (NotificationRepository.ui.value.permission) NotificationRepository.allow(request)
            else NotificationRepository.decline(request)
        }
        pendingNotificationConsent = null
    }

    private val accessibilityObserver = object : ContentObserver(Handler(Looper.getMainLooper())) {
        override fun onChange(selfChange: Boolean) = refresh()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        savedInstanceState?.getString("notificationConsentScope")?.let {
            pendingNotificationConsent = NotificationConsentRequest(it, savedInstanceState.getLong("notificationConsentRevision"))
        }
        snapshotReader = HomeSnapshotReader(this)
        showJustFinished = intent.getBooleanExtra(EXTRA_JUST_FINISHED, false)
        intent.removeExtra(EXTRA_JUST_FINISHED)
        continueStylization = savedInstanceState?.getBoolean(EXTRA_CONTINUE_STYLIZATION)
            ?: intent.getBooleanExtra(EXTRA_CONTINUE_STYLIZATION, false)
        intent.removeExtra(EXTRA_CONTINUE_STYLIZATION)

        setContent {
            GovorunTheme {
                val billing by BillingRepository.state.collectAsStateWithLifecycle()
                val account by GovorunAccountRepository.state.collectAsStateWithLifecycle()
                val notifications by NotificationRepository.ui.collectAsStateWithLifecycle()
                val onlinePaused by com.govorun.lite.stylization.OfficialServiceAvailability
                    .get(this@MainActivity).paused.collectAsStateWithLifecycle()
                HomeScreen(
                    state = uiState,
                    onlineServicePaused = onlinePaused,
                    styleJustEnabled = styleJustEnabled,
                    paymentAlert = subscriptionPaymentAlert(
                        billing.subscription,
                        account.styleAccess,
                    ),
                    onEvent = ::handleEvent,
                    notificationInvitation = if (account.signedIn && notifications.showInvitation &&
                        uiState.stylization.homeCard == com.govorun.lite.ui.home.HomeStylizationCard.STYLE_PICKER &&
                        !uiState.showStarterGuidance) {
                        { NotificationInvitation(
                            onAllow = ::allowNotifications,
                            onLater = NotificationRepository::later,
                        ) }
                    } else null,
                )
            }
        }
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                launch {
                    com.govorun.lite.stylization.StyleProcessingFeedback.state.collect { refresh() }
                }
                GovorunAccountRepository.state.collect {
                    refresh()
                    resumeStylizationAfterLogin()
                }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        NotificationRepository.refresh()
        // Pull the latest cloud snapshot whenever the user returns to the
        // dashboard. Local dictation can continue while this activity is not
        // visible, and another signed-in device may have added new events.
        GovorunAccountRepository.refreshAccount()
        lifecycleScope.launch { BillingRepository.load() }
        refresh()
        resumeStylizationAfterLogin()
        registerAccessibilityObservers()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putBoolean(EXTRA_CONTINUE_STYLIZATION, continueStylization)
        pendingNotificationConsent?.let {
            outState.putString("notificationConsentScope", it.accountScope)
            outState.putLong("notificationConsentRevision", it.revision)
        }
        super.onSaveInstanceState(outState)
    }

    private fun allowNotifications() {
        if (pendingNotificationConsent != null) return
        val request = NotificationRepository.consentRequest() ?: return
        if (NotificationRepository.ui.value.permission) {
            NotificationRepository.allow(request)
        } else if (Build.VERSION.SDK_INT >= 33 && ContextCompat.checkSelfPermission(this,
                Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            pendingNotificationConsent = request
            notificationPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
        } else {
            pendingNotificationConsent = request
            notificationSettings.launch(Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                .putExtra(Settings.EXTRA_APP_PACKAGE, packageName))
        }
    }

    override fun onPause() {
        styleJustEnabled = false
        contentResolver.unregisterContentObserver(accessibilityObserver)
        super.onPause()
    }

    private fun refresh() {
        refreshRequested = true
        if (refreshJob?.isActive == true) return
        refreshJob = lifecycleScope.launch {
            while (refreshRequested) {
                refreshRequested = false
                val justFinished = showJustFinished
                showJustFinished = false
                val next = withContext(Dispatchers.IO) { snapshotReader.read(justFinished) }
                if (next != uiState) uiState = next
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        if (intent.getBooleanExtra(EXTRA_CONTINUE_STYLIZATION, false)) continueStylization = true
        intent.removeExtra(EXTRA_CONTINUE_STYLIZATION)
    }

    private fun resumeStylizationAfterLogin() {
        if (!continueStylization) return
        val account = GovorunAccountRepository.state.value
        if (!account.signedIn) { continueStylization = false; return }
        if (!com.govorun.lite.account.shouldFinishAccountLogin(true, account)) return
        continueStylization = false
        if (uiState.stylization.enabled && uiState.stylization.canEnableDirectly) {
            styleJustEnabled = true
        } else requestStyleEnable(afterSignIn = true)
    }

    private fun requestStyleEnable(style: String? = null, afterSignIn: Boolean = false) {
        StylizationConsent.requestEnable(this, requestedStyle = style, afterSignIn = afterSignIn) { accepted ->
            styleJustEnabled = accepted
            refresh()
        }
    }

    private fun handleEvent(event: HomeEvent) {
        when (event) {
            HomeEvent.OpenSettings -> startActivity(Intent(this, SettingsActivity::class.java))
            HomeEvent.OpenAbout -> startActivity(Intent(this, AboutActivity::class.java))
            HomeEvent.OpenSubscription ->
                startActivity(Intent(this, SubscriptionActivity::class.java))
            HomeEvent.RetryConnection -> {
                GovorunAccountRepository.refreshAccount()
                lifecycleScope.launch { BillingRepository.load() }
            }
            HomeEvent.OpenStylization ->
                startActivity(Intent(this, StylizationActivity::class.java))
            HomeEvent.RequestStylizationConsent ->
                requestStyleEnable()
            HomeEvent.StartStylization ->
                startActivity(Intent(this, AccountActivity::class.java)
                    .putExtra(AccountActivity.EXTRA_RETURN_TO_STYLIZATION, true))
            HomeEvent.OpenDictionary ->
                startActivity(Intent(this, DictionaryActivity::class.java))
            HomeEvent.DisableStylization -> {
                styleJustEnabled = false
                if (StylizationServerPreferences.mode(this) == StylizationServerMode.SELF_HOSTED) {
                    StylizationServerPreferences.setStylizationEnabled(this, false)
                } else {
                    AndroidStylizationSettings.create(this).setEnabled(false)
                }
                refresh()
            }
            HomeEvent.OpenAccount ->
                startActivity(Intent(this, AccountActivity::class.java))
            HomeEvent.MergeAccountData -> GovorunAccountRepository.mergeLocalDataIntoAccount()
            HomeEvent.KeepAccountDataLocal -> GovorunAccountRepository.keepLocalDataOnly()
            HomeEvent.OpenReview ->
                startActivity(Intent(this, FuturePlansActivity::class.java))
            HomeEvent.OpenProducts ->
                startActivity(Intent(this, OurProductsActivity::class.java))
            HomeEvent.OpenAccessibility ->
                AccessibilityHelper.openAccessibilitySettings(this)
            HomeEvent.OpenAppDetails -> openAppDetails()
            HomeEvent.RequestBattery -> requestBatteryExemption()
            HomeEvent.DismissStarterGuidance -> {
                HomePreferences.dismissStarterGuidance(this)
                refresh()
            }
            is HomeEvent.DismissStyleFeedback ->
                com.govorun.lite.stylization.StyleProcessingFeedback.dismiss(event.revision)
            is HomeEvent.SelectStyle -> {
                if (uiState.stylization.selfHosted) return
                if (!uiState.stylization.enabled && uiState.stylization.accessActive) {
                    requestStyleEnable(event.style.apiId)
                    return
                }
                if (event.style.apiId != "custom") {
                    GovorunAccountRepository.recordSyncedContentChange(this)
                }
                Prefs.setStylizationStyle(this, event.style.apiId)
                if (!uiState.stylization.enabled) {
                    if (uiState.stylization.canEnableDirectly) {
                        AndroidStylizationSettings.create(this).setEnabled(true)
                    } else {
                        handleEvent(uiState.stylization.styleSetupEvent)
                        return
                    }
                }
                if (event.style.apiId != "custom") {
                    GovorunAccountRepository.recordPortablePreferenceChange(
                        this,
                        defaultStyle = event.style.apiId,
                    )
                }
                refresh()
            }
            is HomeEvent.SelectCustomStyle -> {
                if (uiState.stylization.selfHosted) {
                    if (!StylizationServerPreferences.selectCustom(this, event.id)) return
                    StylizationServerPreferences.setStylizationEnabled(this, true)
                    refresh()
                    return
                }
                if (!uiState.stylization.enabled && uiState.stylization.accessActive) {
                    requestStyleEnable("custom:${event.id}")
                    return
                }
                if (!AndroidCustomStyles.create(this).select(event.id)) return
                enableStylesFromDashboard()
            }
            HomeEvent.SelectSelfHostedGeneral -> {
                if (!uiState.stylization.selfHosted ||
                    !uiState.stylization.canEnableDirectly
                ) return
                StylizationServerPreferences.selectGeneral(this)
                StylizationServerPreferences.setStylizationEnabled(this, true)
                refresh()
            }
            is HomeEvent.SaveCustomStyle -> {
                val saved = if (uiState.stylization.selfHosted) {
                    SelfHostedStylePresets.save(
                        this, event.id, event.name, event.instruction,
                    )
                } else {
                    AndroidCustomStyles.create(this).saveAndSelect(event.id, event.name, event.instruction,
                    )
                }
                if (saved != null) {
                    if (uiState.stylization.selfHosted) {
                        StylizationServerPreferences.selectCustom(this, saved.id)
                        StylizationServerPreferences.setStylizationEnabled(this, true)
                    }
                    enableStylesFromDashboard()
                }
            }
            is HomeEvent.DeleteCustomStyle -> {
                if (uiState.stylization.selfHosted) {
                    SelfHostedStylePresets.delete(this, event.id)
                } else {
                    AndroidCustomStyles.create(this).delete(event.id)
                }
                if (uiState.stylization.selfHosted &&
                    StylizationServerPreferences.selectedStyle(this) == "custom:${event.id}"
                ) {
                    StylizationServerPreferences.selectGeneral(this)
                }
                refresh()
            }
        }
    }

    private fun enableStylesFromDashboard() {
        if (uiState.stylization.selfHosted) {
            if (uiState.stylization.canEnableDirectly) {
                StylizationServerPreferences.setStylizationEnabled(this, true)
                refresh()
            } else {
                handleEvent(HomeEvent.OpenStylization)
            }
            return
        }
        if (uiState.stylization.enabled) {
            refresh()
        } else if (uiState.stylization.canEnableDirectly) {
            AndroidStylizationSettings.create(this).setEnabled(true)
            refresh()
        } else {
            handleEvent(uiState.stylization.styleSetupEvent)
        }
    }

    private fun registerAccessibilityObservers() {
        val resolver: ContentResolver = contentResolver
        resolver.registerContentObserver(
            Settings.Secure.getUriFor(Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES),
            false,
            accessibilityObserver,
        )
        resolver.registerContentObserver(
            Settings.Secure.getUriFor("accessibility_button_targets"),
            false,
            accessibilityObserver,
        )
        resolver.registerContentObserver(
            Settings.Secure.getUriFor("accessibility_shortcut_target_service"),
            false,
            accessibilityObserver,
        )
    }

    private fun openAppDetails() {
        val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
            data = Uri.fromParts("package", packageName, null)
        }
        runCatching { startActivity(intent) }
    }

    @Suppress("BatteryLife")
    private fun requestBatteryExemption() {
        val direct = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
            data = Uri.fromParts("package", packageName, null)
        }
        runCatching { startActivity(direct) }.onFailure {
            runCatching {
                startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
            }
        }
    }

    companion object {
        const val EXTRA_JUST_FINISHED = "just_finished"
        const val EXTRA_CONTINUE_STYLIZATION = "continue_stylization"
    }
}
