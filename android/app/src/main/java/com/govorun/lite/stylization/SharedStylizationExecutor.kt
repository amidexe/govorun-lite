package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.*
import com.govorun.lite.shared.SharedBusiness
import kotlinx.coroutines.CancellationException
import org.json.JSONArray
import org.json.JSONObject

internal interface SharedStylizationPorts : StylizationPorts {
    fun selfHostedEnabled(): Boolean
    fun storageAvailable(): Boolean
    fun sessionGeneration(): Long
    suspend fun prepareAccess(requiredCredits: Int)
    /** «Офлайн» or roaming refusal: the person's own server is network too. */
    fun networkRefused(): Boolean
}

/** Executes the local Rust use case; no admission or routing policy lives here. */
internal class SharedStylizationExecutor(private val ports: SharedStylizationPorts) {
    suspend fun process(text: String, mode: StylizationInputMode, packageName: String? = null): StylizationResult {
        return try { execute(text, mode, packageName) }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) {
            // An unavailable or malformed local bridge must not lose the transcript.
            StylizationResult(text, usedFallback = true, fallbackReason = StylizationFallbackReason.REQUEST_FAILED)
                .also(ports::recordResult)
        }
    }

    private suspend fun execute(text: String, mode: StylizationInputMode, packageName: String?): StylizationResult {
        val flow = Flow()
        var frame = flow.step(JSONObject().put("kind", "begin").put("streaming", mode == StylizationInputMode.AUTO).put("bypass", false))
        var account: OfficialStylizationSnapshot? = null
        var generation = 0L
        var selection: StylizationSelection? = null
        var response: StylizationResponse? = null
        while (true) {
            try {
                val event = when (frame.getString("action")) {
                    "readDraft" -> JSONObject().put("kind", "draft").put("active", ports.customDraftActive())
                    "readRoute" -> JSONObject().put("kind", "route").put("selfHosted", ports.selfHostedSelected()).put("enabled", ports.selfHostedEnabled())
                    "readOfficial" -> {
                        generation = ports.sessionGeneration()
                        account = ports.officialState()
                        JSONObject().put("kind", "official").put("units", units(text))
                            .put("enabled", account.enabled).put("acceptedTerms", account.acceptedTermsVersion)
                            .put("currentTerms", account.currentTermsVersion).put("access", facts(account, true))
                    }
                    "readConfiguration" -> JSONObject().put("kind", "configuration").put("configured", ports.officialConfigured())
                    "readSelection" -> {
                        selection = ports.resolveStyle(packageName)
                        JSONObject().put("kind", "selection").put("enabled", selection.enabled)
                    }
                    "prepare" -> {
                        ports.prepareAccess(frame.getJSONObject("plan").getInt("requiredCredits"))
                        JSONObject().put("kind", "prepared").put("access",
                            facts(ports.officialState(), generation == ports.sessionGeneration()))
                    }
                    "readNetwork" -> JSONObject().put("kind", "network").put("available", ports.hasInternet())
                    "request" -> {
                        if (generation != ports.sessionGeneration()) throw StylizationRequestFailure(StylizationFallbackReason.ACCOUNT_STATE_CHANGED)
                        response = ports.request(text, mode, checkNotNull(selection), frame.getJSONObject("plan").getInt("characters"))
                        JSONObject().put("kind", "response").put("input", units(text)).put("output", units(response.text))
                    }
                    "selfHosted" -> return if (ports.networkRefused()) {
                        StylizationResult(text, usedFallback = true, fallbackReason = StylizationFallbackReason.OFFLINE)
                            .also(ports::recordResult)
                    } else ports.processSelfHosted(text, mode)
                    "complete" -> {
                        val reason = enumName(frame.getString("reason"))
                        val result = when (frame.getString("outcome")) {
                            "styled" -> {
                                ports.didStyle(checkNotNull(account).starter)
                                val styled = checkNotNull(response)
                                StylizationResult(styled.text, outcomePass = styled.outcomePass, latencyBucket = styled.latencyBucket)
                            }
                            "fallback" -> StylizationResult(text, usedFallback = true,
                                fallbackReason = StylizationFallbackReason.valueOf(reason),
                                outcomePass = response?.outcomePass, latencyBucket = response?.latencyBucket ?: "unknown")
                            else -> StylizationResult(text, skipReason = StylizationSkipReason.valueOf(reason))
                        }
                        ports.recordResult(result)
                        return result
                    }
                    else -> error("Invalid processing instruction")
                }
                frame = flow.step(event)
            } catch (cancelled: CancellationException) { throw cancelled
            } catch (failure: Exception) {
                val reason = (failure as? StylizationRequestFailure)?.reason ?: StylizationFallbackReason.REQUEST_FAILED
                frame = flow.step(JSONObject().put("kind", "failure").put("reason", wireName(reason.name)))
            }
        }
    }

    private class Flow {
        private var state: JSONObject? = null
        fun step(event: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
            .put("operation", "dictationPreflight").put("command", JSONObject().put("kind", "workflow")
                .put("state", state ?: JSONObject.NULL).put("event", event)))
            .also { state = it.getJSONObject("state") }
    }

    private fun facts(value: OfficialStylizationSnapshot, current: Boolean): JSONObject {
        val access = value.accessState
        return JSONObject().put("selected", value.enabled).put("sessionCurrent", current)
            .put("entitlementActive", access != OfficialStylizationAccessState.SIGN_IN_REQUIRED && access != OfficialStylizationAccessState.ENTITLEMENT_REQUIRED)
            .put("processingReady", access == OfficialStylizationAccessState.READY)
            .put("termsRequired", value.acceptedTermsVersion != value.currentTermsVersion ||
                access == OfficialStylizationAccessState.TERMS_VERSION_MISMATCH || access == OfficialStylizationAccessState.ONLINE_CONSENT_REQUIRED)
            .put("serviceAvailable", value.serviceAvailable && access != OfficialStylizationAccessState.PROTOCOL_UNAVAILABLE)
            .put("storageAvailable", ports.storageAvailable()).put("configured", true).put("networkAvailable", JSONObject.NULL)
    }

    private fun units(value: String) = JSONArray(value.map { it.code })
    private fun enumName(value: String) = value.replace(Regex("([a-z])([A-Z])"), "$1_$2").uppercase()
    private fun wireName(value: String) = value.lowercase().split('_').let { it.first() + it.drop(1).joinToString("") { word -> word.replaceFirstChar(Char::uppercaseChar) } }
}
