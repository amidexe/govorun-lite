package com.govorun.lite.shared

import org.json.JSONObject

/** A JNI bridge to the shared Rust implementation; no Android business rules. */
internal object SharedBusiness {
    private val library = SharedLibrary { System.loadLibrary("govorun_jvm_bridge") }

    private external fun evaluateNative(command: String): String

    internal fun requireAvailable() = library.requireAvailable()

    fun evaluate(command: JSONObject): JSONObject = decode { evaluateNative(command.toString()) }

    private fun decode(call: () -> String): JSONObject {
        library.requireAvailable()
        val response = try { JSONObject(call()) }
        catch (_: LinkageError) { throw SharedBusinessException("business_bridge_unavailable", 0) }
        catch (_: Exception) { throw SharedBusinessException("business_bridge_unavailable", 0) }
        if (response.has("error")) {
            throw SharedBusinessException(response.getString("error"), response.optInt("status"),
                response.optJSONObject("clientCheckoutFailure"), response.optJSONObject("clientCheckoutRecoveryFailure"))
        }
        return response.getJSONObject("result")
    }
}

internal class SharedLibrary(load: () -> Unit) {
    private val available = try { load(); true }
        catch (_: LinkageError) { false }
        catch (_: SecurityException) { false }
    fun requireAvailable() {
        if (!available) throw SharedBusinessException("business_bridge_unavailable", 0)
    }
}

internal class SharedBusinessException(val code: String, val status: Int,
    val checkoutFailure: JSONObject? = null, val checkoutRecoveryFailure: JSONObject? = null,
    val stylizationFailure: JSONObject? = null) :
    IllegalArgumentException(code)
