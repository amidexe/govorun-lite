package com.govorun.lite.api

import android.os.ParcelFileDescriptor
import com.govorun.api.v1.IContinuousRecognitionCallback
import com.govorun.api.v1.IRecognitionCallback
import java.io.ByteArrayOutputStream
import java.io.IOException

internal object PcmAudioReader {
    fun read(descriptor: ParcelFileDescriptor): ByteArray {
        val declaredSize = descriptor.statSize
        if (declaredSize < 0L) throw InvalidPcmException()
        when (PcmAudioValidator.sizeError(declaredSize)) {
            RecognitionApi.ERROR_INVALID_AUDIO -> throw InvalidPcmException()
        }
        if (declaredSize > RecognitionApi.IN_MEMORY_AUDIO_BYTES) throw PcmLimitException()
        return try {
            ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { input ->
                val output = ByteArrayOutputStream()
                val buffer = ByteArray(16 * 1024)
                while (true) {
                    val read = input.read(buffer)
                    if (read < 0) break
                    output.write(buffer, 0, read)
                    if (output.size() > RecognitionApi.IN_MEMORY_AUDIO_BYTES) throw PcmLimitException()
                }
                output.toByteArray()
            }
        } catch (error: PcmLimitException) {
            throw error
        } catch (_: IOException) {
            throw InvalidPcmException()
        }
    }
}

internal class PcmLimitException : IllegalArgumentException()
internal class InvalidPcmException : IllegalArgumentException()

internal fun ParcelFileDescriptor.closeQuietly() {
    runCatching { close() }
}

internal fun IRecognitionCallback.error(sessionId: String, code: Int, message: String) {
    runCatching { onError(sessionId, code, message) }
}

internal fun IContinuousRecognitionCallback.error(sessionId: String, code: Int, message: String) {
    runCatching { onError(sessionId, code, message) }
}
