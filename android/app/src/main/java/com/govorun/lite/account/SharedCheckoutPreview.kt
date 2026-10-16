package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal fun sharedCheckoutPreview(wire: JSONObject): JSONObject = SharedBusiness.evaluate(
    JSONObject().put("operation", "checkoutPreview").put("preview", wire),
).getJSONObject("preview")
