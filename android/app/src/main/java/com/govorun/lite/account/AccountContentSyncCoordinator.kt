package com.govorun.lite.account

import org.json.JSONObject

internal class AccountContentSyncCoordinator(
    private val secure: AccountSessionReader,
    private val api: AccountApiClient,
    private val store: AccountContentSyncStore,
) {
    fun markLocalChange(accountID: String?) = store.markLocalChange(accountID)
    fun hasPending(accountID: String): Boolean = store.hasPending(accountID)

    suspend fun synchronize(initial: GovorunSession, accountID: String) {
        if (initial.accountId != accountID) throw AccountSessionChangedException()
        var flow: JSONObject? = null
        var event = JSONObject().put("kind", "start")
        while (true) {
            requireCurrent(initial)
            val decision = store.step(accountID, flow, event)
            flow = decision.optJSONObject("flow")
            event = when (decision.getString("action")) {
                "complete" -> return
                "failed" -> throw AccountHttpException(decision.optInt("status", 0))
                "read", "write" -> try {
                    val response = authorized(initial) { token ->
                        if (decision.getString("action") == "read") {
                            api.syncDocument(token, decision.optString("etag").takeUnless { decision.isNull("etag") })
                        } else {
                            api.putSyncDocument(token, decision.getString("etag"),
                                SyncDocumentJson.decode(decision.getJSONObject("document").toString()))
                        }
                    }
                    ContentWorkflowBinding.response(decision.getString("action"), response)
                } catch (error: AccountHttpException) {
                    JSONObject().put("kind", "failed").put("status", error.statusCode)
                }
                else -> error("Invalid shared content action")
            }
        }
    }

    private suspend fun <T> authorized(
        session: GovorunSession,
        call: suspend (String) -> T,
    ): T {
        requireCurrent(session)
        val result = call(session.accessToken)
        requireCurrent(session)
        return result
    }

    private fun requireCurrent(session: GovorunSession) {
        val current = secure.readableSession() ?: throw AccountSessionChangedException()
        if (!sameLogicalAccount(session, current)) throw AccountSessionChangedException()
    }

}
