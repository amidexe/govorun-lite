package com.govorun.lite.notifications

import org.json.JSONObject
import com.govorun.lite.shared.SharedBusiness

/** Wire categories also preserve the narrower consent of older installations. */
data class NotificationChoices(val updates: Boolean = false, val offers: Boolean = false) {
    val enabled: Boolean get() = notificationCommand(JSONObject().put("kind", "enabled").put("state", wire())).getBoolean("allowed")
    fun accepts(category: String): Boolean = notificationCommand(JSONObject().put("kind", "accepts")
        .put("state", wire()).put("category", category)).getBoolean("allowed")
    internal fun wire() = JSONObject().put("updates", updates).put("offers", offers)
}

data class UpdateNotice(
    val id: String,
    val category: String,
    val title: String,
    val body: String,
    val publishedAt: Long,
    val expiresAt: Long,
) {
    companion object {
        fun parse(data: Map<String, String>, app: String, version: Long, now: Long): UpdateNotice? {
            val command = JSONObject().put("kind", "parse").put("data", JSONObject(data)).put("package", app).put("version", version).put("now", now)
            return notificationValue(command)?.let(::read)
        }
        internal fun read(v: JSONObject) = UpdateNotice(v.getString("id"), v.getString("category"), v.getString("title"),
            v.getString("body"), v.getLong("publishedAt"), v.getLong("expiresAt"))
    }
}

data class NotificationState(
    val accountScope: String = "",
    val firstSeenAt: Long = 0,
    val signedInAt: Long = 0,
    val choices: NotificationChoices = NotificationChoices(),
    val prompted: Boolean = false,
    val id: String = "",
    val credential: String = "",
    val revision: Long = 0,
    val acknowledged: Long = 0,
    val token: String = "",
    val versionCode: Long = 0,
    val syncedAt: Long = 0,
    val sdkRemovalPending: Boolean = false,
    val seen: Map<String, Long> = emptyMap(),
    val notices: Map<String, UpdateNotice> = emptyMap(),
) {
    fun receive(notice: UpdateNotice, permission: Boolean, now: Long): NotificationState? =
        changed("receive", permission = permission, now = now, extra = JSONObject().put("notice", notice.wire()))
    internal fun decision(kind: String, scope: String = "", permission: Boolean = false, now: Long = 0, extra: JSONObject = JSONObject()): JSONObject =
        notificationCommand(command(kind, scope, permission, now, extra))
    internal fun changed(kind: String, scope: String = "", permission: Boolean = false, now: Long = 0, extra: JSONObject = JSONObject()): NotificationState? =
        notificationValue(command(kind, scope, permission, now, extra))?.let(::notificationState)
    private fun command(kind: String, scope: String, permission: Boolean, now: Long, extra: JSONObject) = extra
        .put("kind", kind).put("state", wire()).put("scope", scope).put("permission", permission).put("now", now)
}
internal fun UpdateNotice.wire() = JSONObject().put("id", id).put("category", category).put("title", title)
    .put("body", body).put("publishedAt", publishedAt).put("expiresAt", expiresAt)
internal fun NotificationState.wire(): JSONObject = choices.wire().put("schema", 1).put("accountScope", accountScope)
    .put("firstSeenAt", firstSeenAt).put("signedInAt", signedInAt).put("prompted", prompted).put("id", id)
    .put("credential", credential).put("revision", revision).put("acknowledged", acknowledged).put("token", token)
    .put("versionCode", versionCode).put("syncedAt", syncedAt).put("sdkRemovalPending", sdkRemovalPending)
    .put("seen", JSONObject(seen)).put("notices", JSONObject().apply { notices.forEach { (k, v) -> put(k, v.wire()) } })
internal fun notificationState(v: JSONObject): NotificationState {
    val seen = v.optJSONObject("seen") ?: JSONObject(); val notices = v.optJSONObject("notices") ?: JSONObject()
    return NotificationState(v.optString("accountScope"), v.optLong("firstSeenAt"), v.optLong("signedInAt"),
        NotificationChoices(v.optBoolean("updates"), v.optBoolean("offers")), v.optBoolean("prompted"), v.optString("id"),
        v.optString("credential"), v.optLong("revision"), v.optLong("acknowledged"), v.optString("token"), v.optLong("versionCode"),
        v.optLong("syncedAt"), v.optBoolean("sdkRemovalPending"), seen.keys().asSequence().associateWith { seen.getLong(it) },
        notices.keys().asSequence().filter { it in setOf("updates", "offers") }.associateWith { k ->
            UpdateNotice.read(notices.getJSONObject(k).put("category", k)) })
}
internal fun notificationValue(command: JSONObject): JSONObject? = SharedBusiness.evaluate(JSONObject()
    .put("operation", "optionalNotifications").put("command", command)).optJSONObject("value")
