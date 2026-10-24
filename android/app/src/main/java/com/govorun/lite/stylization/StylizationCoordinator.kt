package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.officialStylizationAccessState
import com.govorun.lite.model.stylization.OfficialStylizationSnapshot
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StylizationClientOutcome
import com.govorun.lite.model.stylization.StylizationFallbackReason
import com.govorun.lite.model.stylization.StylizationInputMode
import com.govorun.lite.model.stylization.StylizationRequestFailure
import com.govorun.lite.model.stylization.StylizationResponse
import com.govorun.lite.model.stylization.StylizationResult
import com.govorun.lite.model.stylization.StylizationSelection
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.HomePreferences
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.CancellationException

/** Android composition and adapters for the SDK-independent processing scenario. */
internal class StylizationCoordinator(
    context: Context,
    client: StylizationClient? = null,
    private val selfHostedClientFactory: (SelfHostedServerSettings) -> StylizationClient =
        { OpenAiCompatibleStylizationClient(it) },
    private val hasInternet: (Context) -> Boolean = NetworkAvailability::hasUsableNetwork,
) {
    private val appContext = context.applicationContext
    private val client = client ?: ConfiguredStylizationClient(appContext)

    suspend fun process(
        recognizedText: String,
        inputMode: StylizationInputMode,
        packageName: String? = null,
    ): StylizationResult = DiagnosticAttempt.collect(appContext) {
        val ports = AndroidPorts()
        val ticket = StyleProcessingFeedback.begin(ports.sessionGeneration(), ports.selfHostedSelected())
        try {
            SharedStylizationExecutor(ports).process(recognizedText, inputMode, packageName).also {
                if (it.skipReason != null) StyleProcessingFeedback.cancel(ticket)
                else StyleProcessingFeedback.finish(ticket,
                    if (it.usedFallback) it.fallbackReason ?: StylizationFallbackReason.REQUEST_FAILED else null)
            }
        } finally { StyleProcessingFeedback.cancel(ticket) }
    }

    suspend fun reportClientOutcome(
        outcomePass: String?, outcome: StylizationClientOutcome, latencyBucket: String,
    ) {
        if (outcomePass == null) return
        try {
            client.reportClientOutcome(outcomePass, outcome, latencyBucket)
        } catch (error: CancellationException) {
            throw error
        } catch (error: Exception) {
            AppLog.log(appContext, "Stylization: client outcome report failed error=${error.javaClass.simpleName}")
        }
    }

    private inner class AndroidPorts : SharedStylizationPorts {
        private var preparationMilliseconds = 0L
        override fun selfHostedEnabled() = StylizationServerPreferences.stylizationEnabled(appContext)
        override fun storageAvailable() = GovorunAccountRepository.state.value.secureStorageProblem == null
        override fun sessionGeneration() = GovorunAccountRepository.state.value.sessionGeneration
        override suspend fun prepareAccess(requiredCredits: Int) {
            preparationMilliseconds = GovorunAccountRepository.joinDictationPreparation(requiredCredits)
        }
        override fun customDraftActive() = CustomStyleDraftSession.isActive()
        // Only the person's refusal counts: a server in the home network keeps working without internet.
        override fun networkRefused() = com.govorun.lite.network.NetworkPolicy.activeRefusal(appContext) != null
        override fun selfHostedSelected() =
            StylizationServerPreferences.mode(appContext) == StylizationServerMode.SELF_HOSTED

        override fun officialState(): OfficialStylizationSnapshot {
            val account = GovorunAccountRepository.state.value
            return OfficialStylizationSnapshot(
                enabled = Prefs.isStylizationEnabled(appContext),
                acceptedTermsVersion = Prefs.getStylizationConsentVersion(appContext),
                currentTermsVersion = StylizationTerms.CURRENT_VERSION,
                accessState = officialStylizationAccessState(account),
                serviceAvailable = account.onlineService.available,
                starter = account.styleAccess.type == "starter",
            )
        }

        override fun officialConfigured() = BuildConfig.OFFICIAL_STYLES_API_URL.isNotBlank()
        override fun hasInternet() = hasInternet(appContext)
        override fun resolveStyle(packageName: String?): StylizationSelection {
            val resolved = StylizationStylePreferences.resolve(appContext, packageName)
            return StylizationSelection(resolved.enabled, resolved.style,
                resolved.customPreset?.instruction ?: if (resolved.style == TextStyle.CUSTOM) {
                    Prefs.getStylizationCustomInstruction(appContext)
                } else null, resolved.options)
        }

        override suspend fun request(text: String, mode: StylizationInputMode,
                                     selection: StylizationSelection, characters: Int): StylizationResponse = try {
            DiagnosticAttempt.current?.recordRequestCharacters(text.length)
            client.stylize(text, selection.style, mode, selection.instruction,
                selection.options.punctuation, selection.options.emojiEnabled,
                selection.options.profanityFilterEnabled, selection.options.resolvedEmojiMode,
                preparationMillis = preparationMilliseconds)
        } catch (error: CancellationException) {
            throw error
        } catch (error: Exception) {
            throw StylizationRequestFailure(requestFailureReason(error))
        }

        override suspend fun processSelfHosted(text: String, mode: StylizationInputMode): StylizationResult {
            val settings = StylizationServerPreferences.settings(appContext)
            return SelfHostedProcessing.process(text,
                record = { LocalDiagnosticJournal.recordTiming(appContext, it.encode()) },
            ) {
                require(settings.isComplete && settings.hasConsent)
                selfHostedClientFactory(settings).stylize(text = text, style = TextStyle.CUSTOM,
                    inputMode = mode, instruction = settings.generalPrompt,
                    punctuationMode = PunctuationMode.NORMAL, emojiEnabled = false,
                    profanityFilterEnabled = false)
            }
        }

        override fun didStyle(starter: Boolean) {
            if (starter) HomePreferences.dismissStarterGuidance(appContext)
        }

        override fun recordResult(result: StylizationResult) {
            val outcome = when {
                result.usedFallback -> StylizationDiagnosticOutcome.FALLBACK
                result.skipReason != null -> StylizationDiagnosticOutcome.SKIPPED
                else -> StylizationDiagnosticOutcome.STYLED
            }
            val reason = result.fallbackReason?.let(::diagnosticReason)
                ?: result.skipReason?.let { StylizationDiagnosticReason.valueOf(it.name) }
                ?: StylizationDiagnosticReason.SUCCESS
            StylizationDiagnosticsStore.record(appContext, outcome, reason)
            GovorunAccountRepository.recordStyleFailure(reason.apiId)
            AppLog.log(appContext, "Stylization: outcome=${outcome.name} reason=${reason.name}")
        }
    }
}
