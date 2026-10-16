package com.govorun.lite.account

import org.json.JSONObject

/** The shared recovery owner decides whether discovery is empty. */
internal data class CheckoutRecoveryFailure(val discoveryEmpty: Boolean, val message: String?) {
    companion object {
        fun read(value: JSONObject): CheckoutRecoveryFailure {
            val empty = value.get("discoveryEmpty") as Boolean
            val message = if (value.isNull("message")) null else value.get("message") as String
            require(empty == (message == null) && (message == null || message.isNotBlank()))
            return CheckoutRecoveryFailure(empty, message)
        }
    }
}

internal class CheckoutRecoveryException(val failure: CheckoutRecoveryFailure) :
    IllegalStateException(failure.message)
