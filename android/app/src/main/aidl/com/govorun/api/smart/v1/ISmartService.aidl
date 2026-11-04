package com.govorun.api.smart.v1;

import android.app.PendingIntent;
import com.govorun.api.smart.v1.ISmartCallback;

/**
 * Govorun's bridge for Говорун Эхо (protocol 3). Bind with the action com.govorun.api.smart.v1.BIND.
 * Every call names the calling package, which must match the Binder UID, be Echo by its signature and
 * have the recognition access the person gave it in Govorun (Settings → API для приложений).
 */
interface ISmartService {
    /** 3. */
    int getProtocolVersion();

    /**
     * JSON answered from Govorun's memory, never from the network: {"state", "message", "textTasks",
     * "styles", "volumeKey", "capabilities", "smart"}. state: ready, not_authorized, not_signed_in, offline,
     * roaming, no_network. "smart": the person once pressed a smart function in the app. Unauthorized callers
     * learn nothing about the account.
     */
    String getStatus(String clientPackage);

    /** A Govorun screen: sign_in, subscription, network, authorize, stylization, volume_key. */
    PendingIntent createAction(String clientPackage, String action);

    /**
     * kind: text_task, inputJson {"operation", "text", "requestClass", "style"?, "asked"}.
     * Govorun forwards the opaque operation through the common paid pipeline; only the server owns its catalog.
     * text_task_price {"text", "requestClass"} is a local quote. collect {} only reads the same accepted taskId;
     * it never starts a missing task, asks for a grant or submits text again. Exactly one callback per call.
     * An accepted task may wait in Govorun (onError "queued"); TASKS_CHANGED announces its updated state.
     * "asked": true means the person pressed it; automatic work requires a previous explicit smart action.
     */
    void runTask(String clientPackage, String taskId, String kind, String inputJson, ISmartCallback callback);

    void cancelTask(String clientPackage, String taskId);

    /**
     * The app's tasks Govorun keeps, as JSON [{"taskId", "kind", "state", "wait", "message"}]; state: waiting,
     * done, failed. Unauthorized callers get [].
     */
    String getTasks(String clientPackage);
}
