package com.govorun.lite.model

import android.content.Context
import java.io.File

/**
 * Hardcoded config for GigaAM v3 E2E RNNT (Sber, MIT license).
 * End-to-end Russian ASR with punctuation and capitalization. WER ~8.4%.
 *
 * Upstream source: github.com/salute-developers/GigaAM
 *
 * Model files ship inside the APK as assets (total ~310 MB). On first launch,
 * [ensureInstalled] copies them to [modelDir] under filesDir because
 * sherpa-onnx wants real file paths, not AssetManager descriptors. The
 * bundle-and-copy pattern replaced an earlier remote-download flow that
 * routinely timed out — GitHub now serves release assets via Azure Blob,
 * which is painfully slow from Russia, and RuStore moderators kept failing
 * the download step.
 */
object GigaAmModel {

    const val ENCODER = "gigaam_v3_e2e_rnnt_encoder_int8.onnx"
    const val DECODER = "gigaam_v3_e2e_rnnt_decoder.onnx"
    const val JOINER  = "gigaam_v3_e2e_rnnt_joint.onnx"
    const val TOKENS  = "gigaam_v3_e2e_rnnt_tokens.txt"

    data class ModelFile(val name: String, val sizeBytes: Long)

    private const val ASSETS_SUBDIR = "models/gigaam-v3"

    val FILES = listOf(
        ModelFile(ENCODER, 318_995_997L),
        ModelFile(DECODER, 4_600_058L),
        ModelFile(JOINER,  2_712_896L),
        ModelFile(TOKENS,  13_353L),
    )

    val TOTAL_BYTES: Long = FILES.sumOf { it.sizeBytes }

    fun modelDir(context: Context): File =
        File(context.filesDir, "models/gigaam-v3-e2e-rnnt").also { it.mkdirs() }

    /** True when all expected files exist on disk with the expected sizes. */
    fun isInstalled(context: Context): Boolean {
        val dir = modelDir(context)
        return FILES.all {
            val f = File(dir, it.name)
            f.exists() && f.length() == it.sizeBytes
        }
    }

    /**
     * Copies bundled model files from APK assets to internal storage if
     * they aren't there yet. Safe to call on any thread but blocks for the
     * duration of the copy (~5-15s on first install, ~0ms afterwards).
     *
     * Copy errors are thrown to the caller so a background crash-report can
     * surface the issue rather than silently leaving the model missing.
     */
    fun ensureInstalled(context: Context) {
        if (isInstalled(context)) return
        val dir = modelDir(context)
        val assets = context.assets
        for (file in FILES) {
            val target = File(dir, file.name)
            if (target.exists() && target.length() == file.sizeBytes) continue
            assets.open("$ASSETS_SUBDIR/${file.name}").use { input ->
                target.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
        }
    }
}
