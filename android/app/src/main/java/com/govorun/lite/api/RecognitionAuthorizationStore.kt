package com.govorun.lite.api

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.security.SecureRandom
import java.util.Base64

internal data class ApprovalRecord(
    val packageName: String,
    val currentSigners: Set<String>,
    val signingLineage: Set<String>,
    val approvedAt: Long,
    val lastUsedAt: Long,
    val scopes: Int,
)

internal data class PendingApproval(
    val nonce: String,
    val uid: Int,
    val packageName: String,
    val currentSigners: Set<String>,
    val expiresAt: Long,
)

internal class RecognitionAuthorizationStore(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun isAuthorized(
        identity: CallerIdentity,
        requiredScope: Int = RecognitionApi.SCOPE_LOCAL_RECOGNITION,
    ): Boolean {
        val record = approval(identity.packageName) ?: return false
        if (!AuthorizationPolicy.signersMatch(record.currentSigners, identity.currentSigners, identity.signingLineage)) {
            revoke(identity.packageName)
            return false
        }
        if (record.scopes and requiredScope != requiredScope) return false
        prefs.edit().putString(approvalKey(identity.packageName), record.copy(lastUsedAt = System.currentTimeMillis()).json()).apply()
        return true
    }

    fun createPending(identity: CallerIdentity): PendingApproval {
        cleanupExpiredPending()
        val previousForUid =
            prefs.all
                .filterKeys { it.startsWith(PENDING_PREFIX) }
                .filterValues { value -> (value as? String)?.let(::pendingFromJson)?.uid == identity.uid }
                .keys
        if (previousForUid.isNotEmpty()) {
            prefs.edit().also { editor -> previousForUid.forEach(editor::remove) }.apply()
        }
        val nonceBytes = ByteArray(24).also(SecureRandom()::nextBytes)
        val request =
            PendingApproval(
                nonce = Base64.getUrlEncoder().withoutPadding().encodeToString(nonceBytes),
                uid = identity.uid,
                packageName = identity.packageName,
                currentSigners = identity.currentSigners,
                expiresAt = System.currentTimeMillis() + PENDING_TTL_MILLIS,
            )
        prefs.edit().putString(pendingKey(request.nonce), request.json()).apply()
        return request
    }

    fun pending(nonce: String): PendingApproval? {
        val key = pendingKey(nonce)
        val raw = prefs.getString(key, null)
        val request = raw?.let(::pendingFromJson) ?: return null
        if (request.expiresAt < System.currentTimeMillis()) {
            prefs.edit().remove(key).apply()
            return null
        }
        return request
    }

    fun consumePending(nonce: String): PendingApproval? {
        val request = pending(nonce) ?: return null
        prefs.edit().remove(pendingKey(nonce)).apply()
        return request
    }

    fun approve(identity: CallerIdentity) {
        val now = System.currentTimeMillis()
        val record =
            ApprovalRecord(
                identity.packageName,
                identity.currentSigners,
                identity.signingLineage,
                now,
                now,
                RecognitionApi.SCOPE_LOCAL_RECOGNITION,
            )
        prefs.edit().putString(approvalKey(identity.packageName), record.json()).apply()
    }

    fun approvedPackages(): List<String> =
        prefs.all.keys
            .asSequence()
            .filter { it.startsWith(APPROVAL_PREFIX) }
            .map { it.removePrefix(APPROVAL_PREFIX) }
            .sorted()
            .toList()

    fun revoke(packageName: String) {
        prefs.edit().remove(approvalKey(packageName)).apply()
    }

    private fun approval(packageName: String): ApprovalRecord? =
        prefs.getString(approvalKey(packageName), null)?.let(::approvalFromJson)

    private fun cleanupExpiredPending() {
        val now = System.currentTimeMillis()
        val expired =
            prefs.all
                .filterKeys { it.startsWith(PENDING_PREFIX) }
                .filterValues { value -> (value as? String)?.let(::pendingFromJson)?.expiresAt?.let { it < now } != false }
                .keys
        if (expired.isNotEmpty()) prefs.edit().also { editor -> expired.forEach(editor::remove) }.apply()
    }

    private fun ApprovalRecord.json(): String =
        JSONObject()
            .put("package", packageName)
            .put("current", currentSigners.jsonArray())
            .put("lineage", signingLineage.jsonArray())
            .put("approvedAt", approvedAt)
            .put("lastUsedAt", lastUsedAt)
            .put("scopes", scopes)
            .toString()

    private fun PendingApproval.json(): String =
        JSONObject()
            .put("nonce", nonce)
            .put("uid", uid)
            .put("package", packageName)
            .put("current", currentSigners.jsonArray())
            .put("expiresAt", expiresAt)
            .toString()

    private fun approvalFromJson(raw: String): ApprovalRecord? =
        runCatching {
            JSONObject(raw).let {
                ApprovalRecord(
                    packageName = it.getString("package"),
                    currentSigners = it.getJSONArray("current").strings(),
                    signingLineage = it.getJSONArray("lineage").strings(),
                    approvedAt = it.getLong("approvedAt"),
                    lastUsedAt = it.getLong("lastUsedAt"),
                    // Approval records created before scoped access existed
                    // granted local recognition only. They never inherit a
                    // future online/stylization scope.
                    scopes = it.optInt("scopes", RecognitionApi.SCOPE_LOCAL_RECOGNITION),
                )
            }
        }.getOrNull()

    private fun pendingFromJson(raw: String): PendingApproval? =
        runCatching {
            JSONObject(raw).let {
                PendingApproval(
                    nonce = it.getString("nonce"),
                    uid = it.getInt("uid"),
                    packageName = it.getString("package"),
                    currentSigners = it.getJSONArray("current").strings(),
                    expiresAt = it.getLong("expiresAt"),
                )
            }
        }.getOrNull()

    private fun Set<String>.jsonArray() = JSONArray().also { array -> sorted().forEach(array::put) }

    private fun JSONArray.strings(): Set<String> =
        buildSet { for (index in 0 until length()) add(getString(index)) }

    private fun approvalKey(packageName: String) = APPROVAL_PREFIX + packageName
    private fun pendingKey(nonce: String) = PENDING_PREFIX + nonce

    private companion object {
        const val PREFS_NAME = "recognition_api_authorization_v1"
        const val APPROVAL_PREFIX = "approval."
        const val PENDING_PREFIX = "pending."
        const val PENDING_TTL_MILLIS = 10 * 60 * 1000L
    }
}

internal object AuthorizationPolicy {
    fun signersMatch(
        approvedCurrent: Set<String>,
        installedCurrent: Set<String>,
        installedLineage: Set<String>,
    ): Boolean {
        if (approvedCurrent.isEmpty() || installedCurrent.isEmpty()) return false
        if (approvedCurrent.size > 1 || installedCurrent.size > 1) return approvedCurrent == installedCurrent
        return approvedCurrent == installedCurrent || approvedCurrent.single() in installedLineage
    }
}
