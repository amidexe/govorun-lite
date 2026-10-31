package com.govorun.api.smart.v1;

import android.app.PendingIntent;
import com.govorun.api.smart.v1.ISmartCallback;

/**
 * Govorun's bridge for Говорун.Эхо (protocol 2). Bind with the action com.govorun.api.smart.v1.BIND.
 * Every call names the calling package, which must match the Binder UID, be Echo by its signature and
 * have the recognition access the person gave it in Govorun (Settings → API для приложений).
 */
interface ISmartService {
    /** 2. */
    int getProtocolVersion();

    /**
     * JSON answered from Govorun's memory, never from the network: {"state", "message", "improve",
     * "styles", "volumeKey", "capabilities", "smart"}. state: ready, not_authorized, not_signed_in, offline,
     * roaming, no_network. "smart": the person once pressed a smart function in the app. Unauthorized callers
     * learn nothing about the account.
     */
    String getStatus(String clientPackage);

    /** A Govorun screen: sign_in, subscription, network, authorize, stylization, volume_key. */
    PendingIntent createAction(String clientPackage, String action);

    /**
     * kind: improve, inputJson {"text", "style"?, "asked"}: the text through Govorun's own text improvement;
     * digest: итоги of {"text", "asked"}; digest_price: their price of {"text"}. Exactly one callback per call. A
     * task that cannot be answered now is kept and tried again by Govorun (onError "queued" with the reason in
     * words); the app hears com.govorun.api.smart.v1.TASKS_CHANGED and asks again with the same task to collect
     * the answer. "asked": true — the person pressed it; false — the app's own task, which goes only after the
     * person once pressed a smart function in the app (smart_off otherwise).
     */
    void runTask(String clientPackage, String taskId, String kind, String inputJson, ISmartCallback callback);

    void cancelTask(String clientPackage, String taskId);

    /**
     * Protocol 2: the app's tasks Govorun keeps, as JSON [{"taskId", "kind", "state", "wait", "message"}]; state: waiting,
     * done, failed. Unauthorized callers get [].
     */
    String getTasks(String clientPackage);
}
