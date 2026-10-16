package com.govorun.lite.stats

import android.content.Context
import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject
import java.time.LocalDate

/** Atomic persistence of the shared state and its ready-to-display projection. */
internal object StatisticsJournal {
    private const val FILE = "govorun_statistics_v1"
    private const val KEY = "journal"

    fun evaluate(state: Any, event: JSONObject, continuity: JSONObject? = null): JSONObject = SharedBusiness.evaluate(
        JSONObject().put("operation", "statisticsLedger")
            .put("command", JSONObject().put("state", state).put("event", event)
                .put("continuity", continuity ?: JSONObject.NULL)))

    fun snapshot(context: Context): JSONObject = synchronized(StatsSyncBarrier.lock) {
        val raw = context.applicationContext.getSharedPreferences(FILE, Context.MODE_PRIVATE)
            .getString(KEY, null)
        val snapshot = if (raw == null) evaluate(JSONObject.NULL, JSONObject().put("kind", "inspect"))
        else JSONObject(raw).also {
            require(it.getInt("schemaVersion") == 1)
            require(it.getJSONObject("state").getInt("schemaVersion") == 1)
            it.getJSONObject("projection")
        }
        if (snapshot.getJSONObject("state").optBoolean("continuityImported")) snapshot
        else evaluate(snapshot, JSONObject().put("kind", "inspect"),
            StatisticsContinuity.read(context, null, LocalDate.now().toString()))
    }

    fun step(context: Context, event: JSONObject): JSONObject = synchronized(StatsSyncBarrier.lock) {
        val snapshot = snapshot(context)
        val state = snapshot.getJSONObject("state")
        val account = event.optString("account").takeIf { it.isNotEmpty() && it != "null" }
        val needsAccount = account != null && state.getJSONObject("accounts")
            .optJSONObject(account)?.optBoolean("continuityImported") != true
        val previous = if (!state.optBoolean("continuityImported") || needsAccount)
            StatisticsContinuity.read(context, account, event.optString("today", LocalDate.now().toString())) else null
        val decision = evaluate(snapshot, event, previous)
        val saved = JSONObject().put("schemaVersion", 1).put("state", decision.getJSONObject("state"))
            .put("projection", decision.getJSONObject("projection"))
        check(context.applicationContext.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY, saved.toString()).commit())
        decision
    }

    fun projection(context: Context, accountId: String? = null): JSONObject {
        val views = snapshot(context).getJSONObject("projection")
        return accountId?.let { views.getJSONObject("accounts").optJSONObject(it) }
            ?: views.getJSONObject("local")
    }
}
