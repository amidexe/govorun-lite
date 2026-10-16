// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class SubscriptionInteractionAction(val wireValue: String) {
    AUTHENTICATE("authenticate"),
    START_FREE_ACCESS("startFreeAccess"),
    RESUME_FREE_ACCESS("resumeFreeAccess"),
    FREE_ACCESS_COMPLETED("freeAccessCompleted"),
    CHOOSE_PLAN("choosePlan");
    companion object {
        fun fromWire(value: String): SubscriptionInteractionAction? =
            entries.firstOrNull { it.wireValue == value }
    }
}
