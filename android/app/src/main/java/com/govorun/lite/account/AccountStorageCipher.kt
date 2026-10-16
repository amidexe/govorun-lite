package com.govorun.lite.account

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import java.security.SecureRandom
import java.util.concurrent.ConcurrentHashMap
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

/** Keystore I/O shared by the existing vault and the account protocol adapter. */
internal interface AccountRecordCipher {
    fun encrypt(value: String): String
    fun decrypt(value: String): String
    fun decryptPreviousUserRecord(value: String): String = decrypt(value)
}

internal class AccountStorageCipher(private val alias: String) : AccountRecordCipher {
    private val keys = processKeys.computeIfAbsent(alias) { Keys() }

    override fun encrypt(value: String): String = synchronized(keys) {
        val data = keys.data ?: newDataKey().also { keys.data = it }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, data.key)
        cipher.updateAAD(recordContext())
        val body = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        "v2.${data.wrapped}.${encode(cipher.iv)}.${encode(body)}"
    }

    override fun decrypt(value: String): String = synchronized(keys) {
        require(value.startsWith("v2."))
        val parts = value.split('.')
        require(parts.size == 5)
        val wrapped = "${parts[1]}.${parts[2]}"
        val data = keys.data?.takeIf { it.wrapped == wrapped } ?: run {
            val bytes = open(secretKey(), parts[1], parts[2], keyContext())
            require(bytes.size == 32)
            DataKey(wrapped, SecretKeySpec(bytes, "AES")).also { keys.data = it }
        }
        open(data.key, parts[3], parts[4], recordContext()).toString(Charsets.UTF_8)
    }

    // Installation identity and pending payments survive old test-session resets.
    override fun decryptPreviousUserRecord(value: String): String = synchronized(keys) {
        if (value.startsWith("v2.")) return decrypt(value)
        val parts = value.split('.')
        require(parts.size == 2)
        open(secretKey(), parts[0], parts[1], null).toString(Charsets.UTF_8)
    }

    private fun newDataKey(): DataKey {
        val bytes = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, secretKey())
        cipher.updateAAD(keyContext())
        return DataKey("${encode(cipher.iv)}.${encode(cipher.doFinal(bytes))}", SecretKeySpec(bytes, "AES"))
    }

    private fun open(key: SecretKey, iv: String, body: String, aad: ByteArray?): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, decode(iv)))
        aad?.let(cipher::updateAAD)
        return cipher.doFinal(decode(body))
    }

    private fun keyContext() = "govorun-account-key-v2:$alias".toByteArray(Charsets.UTF_8)
    private fun recordContext() = "govorun-account-record-v2:$alias".toByteArray(Charsets.UTF_8)

    private fun secretKey(): SecretKey {
        keys.hardware?.let { return it }
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(alias, null) as? SecretKey)?.let { keys.hardware = it; return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
            init(
                KeyGenParameterSpec.Builder(
                    alias,
                    KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
                )
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .build(),
            )
            generateKey().also { keys.hardware = it }
        }
    }

    private fun encode(value: ByteArray): String = Base64.encodeToString(
        value,
        Base64.NO_WRAP or Base64.NO_PADDING or Base64.URL_SAFE,
    )

    private fun decode(value: String): ByteArray = Base64.decode(value, Base64.URL_SAFE)

    private data class DataKey(val wrapped: String, val key: SecretKey)
    private class Keys { var hardware: SecretKey? = null; var data: DataKey? = null }
    private companion object {
        val processKeys = ConcurrentHashMap<String, Keys>()
        const val TRANSFORMATION = "AES/GCM/NoPadding"
    }
}
