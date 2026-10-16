package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal object ContentWorkflowBinding {
    fun step(state: Any, flow: JSONObject?, local: SyncedContentDocument, event: JSONObject): JSONObject =
        SharedBusiness.evaluate(JSONObject().put("operation", "contentWorkflow").put("command", JSONObject()
            .put("state", state).put("flow", flow ?: JSONObject.NULL)
            .put("local", SyncDocumentJson.encode(local)).put("event", event)
            .put("nowMillis", System.currentTimeMillis())))

    fun response(action: String, response: SyncDocumentResponse): JSONObject = JSONObject()
        .put("kind", if (action == "read") "read" else "written")
        .put("etag", response.etag).put("document", SyncDocumentJson.encode(response.document)).apply {
            if (action == "read") {
                put("notModified", response.notModified)
                put("configured", response.configured)
            }
        }
}
