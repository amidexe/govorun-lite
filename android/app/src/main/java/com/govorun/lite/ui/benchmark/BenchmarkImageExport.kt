package com.govorun.lite.ui.benchmark

import android.content.ClipData
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.net.Uri
import android.os.Environment
import android.provider.MediaStore
import androidx.core.content.FileProvider
import java.io.File
import java.io.OutputStream

/** Runs at the Activity's IO boundary; the caller retains ownership of the capture. */
internal class BenchmarkImageExport(private val context: Context) {
    fun shareIntent(bitmap: Bitmap, text: String): Intent {
        val directory = File(context.cacheDir, "benchmark").apply { mkdirs() }
        val image = File.createTempFile("govorun-speed-", ".png", directory)
        try {
            image.outputStream().use { bitmap.writeSharePng(it) }
            val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", image)
            return Intent(Intent.ACTION_SEND).apply {
                type = "image/png"
                putExtra(Intent.EXTRA_STREAM, uri)
                putExtra(Intent.EXTRA_TEXT, text)
                clipData = ClipData.newRawUri("Говорун — результат теста", uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
        } catch (error: Exception) {
            image.delete()
            throw error
        }
    }

    fun saveToGallery(bitmap: Bitmap): Uri {
        val resolver = context.contentResolver
        val values = ContentValues().apply {
            put(MediaStore.Images.Media.DISPLAY_NAME, "Говорун-тест-${System.currentTimeMillis()}.png")
            put(MediaStore.Images.Media.MIME_TYPE, "image/png")
            put(MediaStore.Images.Media.RELATIVE_PATH, "${Environment.DIRECTORY_PICTURES}/Говорун")
            put(MediaStore.Images.Media.IS_PENDING, 1)
        }
        val uri = checkNotNull(resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values))
        try {
            checkNotNull(resolver.openOutputStream(uri)).use { bitmap.writeSharePng(it) }
            values.clear()
            values.put(MediaStore.Images.Media.IS_PENDING, 0)
            check(resolver.update(uri, values, null, null) == 1)
            return uri
        } catch (error: Exception) {
            resolver.delete(uri, null, null)
            throw error
        }
    }
}

private fun Bitmap.writeSharePng(stream: OutputStream) {
    val opaque = withOpaqueShareBackground()
    try {
        check(opaque.compress(Bitmap.CompressFormat.PNG, 100, stream))
    } finally {
        opaque.recycle()
    }
}

internal fun Bitmap.withOpaqueShareBackground(): Bitmap {
    // Compose's GraphicsLayer capture can be HARDWARE. A bitmap-backed Canvas
    // cannot draw it: make a CPU-readable copy before flattening transparency.
    val source = if (config == Bitmap.Config.HARDWARE) {
        checkNotNull(copy(Bitmap.Config.ARGB_8888, false))
    } else this
    try {
        val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        try {
            Canvas(output).apply {
                drawColor(BENCHMARK_SHARE_BACKGROUND_ARGB.toInt())
                drawBitmap(source, 0f, 0f, null)
            }
            return output
        } catch (error: Exception) {
            output.recycle()
            throw error
        }
    } finally {
        if (source !== this) source.recycle()
    }
}
