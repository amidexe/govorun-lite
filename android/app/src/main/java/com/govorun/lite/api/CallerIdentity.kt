package com.govorun.lite.api

import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import java.security.MessageDigest

internal data class CallerIdentity(
    val uid: Int,
    val packageName: String,
    val label: String,
    val currentSigners: Set<String>,
    val signingLineage: Set<String>,
)

internal class CallerIdentityResolver(private val context: Context) {
    fun resolve(uid: Int, packageName: String): CallerIdentity? {
        if (packageName.isBlank()) return null
        val packages = context.packageManager.getPackagesForUid(uid)?.toSet().orEmpty()
        if (packageName !in packages) return null
        return resolveInstalled(packageName)?.takeIf { it.uid == uid }
    }

    @Suppress("DEPRECATION")
    fun resolveInstalled(packageName: String): CallerIdentity? =
        runCatching {
            val packageInfo =
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    context.packageManager.getPackageInfo(
                        packageName,
                        PackageManager.PackageInfoFlags.of(PackageManager.GET_SIGNING_CERTIFICATES.toLong()),
                    )
                } else {
                    context.packageManager.getPackageInfo(packageName, PackageManager.GET_SIGNING_CERTIFICATES)
                }
            val appInfo = packageInfo.applicationInfo ?: return null
            val signingInfo = packageInfo.signingInfo ?: return null
            val current = signingInfo.apkContentsSigners.mapTo(linkedSetOf()) { it.toByteArray().sha256() }
            if (current.isEmpty()) return null
            val lineage =
                if (signingInfo.hasMultipleSigners()) current
                else signingInfo.signingCertificateHistory.mapTo(linkedSetOf()) { it.toByteArray().sha256() }
            CallerIdentity(
                uid = appInfo.uid,
                packageName = packageName,
                label = context.packageManager.getApplicationLabel(appInfo).toString(),
                currentSigners = current,
                signingLineage = lineage,
            )
        }.getOrNull()

    private fun ByteArray.sha256(): String =
        MessageDigest.getInstance("SHA-256").digest(this).joinToString("") { "%02x".format(it) }
}
