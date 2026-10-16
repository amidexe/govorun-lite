package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal fun requireValidSyncedContent(document: SyncedContentDocument) {
    SharedBusiness.evaluate(JSONObject().put("operation", "contentSync").put("command", JSONObject()
        .put("kind", "validate").put("document", SyncDocumentJson.encode(document))))
}
