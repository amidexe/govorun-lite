package com.govorun.lite.api.smart

import com.govorun.lite.BuildConfig
import com.govorun.lite.api.CallerIdentity

/**
 * Говорун.Эхо, the one app the smart functions serve: the person's account, its day of credits and the server's prompts are not
 * offered to other apps, which get the offline recognition API only. Echo is known by its package and its release signature, so
 * no window asks the person about it again; a debug Govorun knows the debug Echo by its package, as the two are built together.
 */
internal object EchoApp {
    private const val PACKAGE = "com.govorun.echo"
    private const val SIGNER = "d87e343df397da98d77612e12c467a8be24e8051d31172298ef728773d634058"

    fun matches(identity: CallerIdentity): Boolean =
        (identity.packageName == PACKAGE && SIGNER in identity.currentSigners) ||
            (BuildConfig.DEBUG && identity.packageName == "$PACKAGE.debug")

    fun matches(packageName: String): Boolean = packageName == PACKAGE || (BuildConfig.DEBUG && packageName == "$PACKAGE.debug")

    /** The person let Echo use Govorun: only then does Govorun's account show Echo's own limit. */
    fun connected(context: android.content.Context): Boolean =
        runCatching { com.govorun.lite.api.RecognitionAuthorizationStore(context).approvedPackages().any(::matches) }.getOrDefault(false)
}
