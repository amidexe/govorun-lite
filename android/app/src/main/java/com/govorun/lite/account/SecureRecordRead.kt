package com.govorun.lite.account

internal sealed interface SecureRecordRead<out T> {
    data object Missing : SecureRecordRead<Nothing>
    data class Available<T>(val value: T) : SecureRecordRead<T>
    data class Unavailable(val problem: SecureStorageProblem) : SecureRecordRead<Nothing>
}

enum class SecureStorageProblem { TEMPORARILY_UNAVAILABLE, MALFORMED }

internal class SecureStorageUnavailableException : Exception("Secure storage unavailable")

/** Read failures never mutate storage and never establish an absent identity. */
internal fun <T> readSecureRecord(
    encrypted: () -> String?,
    decrypt: (String) -> String,
    decode: (String) -> T,
): SecureRecordRead<T> {
    val stored = try { encrypted() } catch (_: Exception) {
        return SecureRecordRead.Unavailable(SecureStorageProblem.TEMPORARILY_UNAVAILABLE)
    } ?: return SecureRecordRead.Missing
    val clear = try { decrypt(stored) } catch (_: Exception) {
        return SecureRecordRead.Unavailable(SecureStorageProblem.TEMPORARILY_UNAVAILABLE)
    }
    return try { SecureRecordRead.Available(decode(clear)) } catch (_: Exception) {
        SecureRecordRead.Unavailable(SecureStorageProblem.MALFORMED)
    }
}

internal val SecureStorageProblem.userMessage: String
    get() = when (this) {
        SecureStorageProblem.TEMPORARILY_UNAVAILABLE ->
            "Не удалось прочитать сохранённый вход. Диктовка работает без стилизации. Повторите попытку."
        SecureStorageProblem.MALFORMED ->
            "Не удалось восстановить сохранённый вход. Данные сохранены. Повторите попытку."
    }

internal inline fun withSecureStorageRead(onUnavailable: () -> Unit, action: () -> Unit) {
    try { action() } catch (_: SecureStorageUnavailableException) { onUnavailable() }
}
