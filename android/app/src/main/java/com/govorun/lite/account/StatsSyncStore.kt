package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.stats.StatisticsJournal
import org.json.JSONObject

/** Account-scoped value binding to the same atomic journal used for local recording. */
internal class StatsSyncStore(context: Context) {
    private val app = context.applicationContext
    fun step(event: JSONObject): JSONObject = StatisticsJournal.step(app, event)
    fun hasPending(accountId: String): Boolean = StatisticsJournal.projection(app, accountId).getBoolean("hasPending")
    // Optional local diagnostics are not evidence of a previous cloud sync.
    fun hasAccountHistory(accountId: String): Boolean = cloudTotal(accountId) != null || isResetPending(accountId)
    fun isResetPending(accountId: String): Boolean = account(accountId)?.getBoolean("resetPending") ?: false
    fun reset(accountId: String?) {
        step(JSONObject().put("kind", "reset").put("account", accountId ?: JSONObject.NULL))
    }
    fun cloudDays(accountId: String): List<StatsDay> {
        val view = StatisticsJournal.projection(app, accountId)
        if (!view.getBoolean("hasCloud")) return emptyList()
        val days = view.getJSONArray("days")
        return List(days.length()) { index -> days.getJSONObject(index).let {
            StatsDay(it.getString("date"), it.getLong("words"), it.getLong("speechSeconds"), it.getLong("recognitions"))
        } }
    }
    fun cloudTotal(accountId: String): StatsLifetimeTotal? {
        val view = StatisticsJournal.projection(app, accountId)
        if (!view.getBoolean("hasCloud")) return null
        val total = view.getJSONObject("total")
        return StatsLifetimeTotal(total.getLong("words"), total.getLong("speechSeconds"), total.getLong("recognitions"))
    }
    private fun account(accountId: String): JSONObject? = StatisticsJournal.snapshot(app)
        .getJSONObject("state").getJSONObject("accounts").optJSONObject(accountId)
}
