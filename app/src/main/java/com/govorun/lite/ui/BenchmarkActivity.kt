package com.govorun.lite.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.os.SystemClock
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButton
import com.google.android.material.color.DynamicColors
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.android.material.progressindicator.LinearProgressIndicator
import com.google.android.material.textview.MaterialTextView
import com.govorun.lite.R
import com.govorun.lite.transcriber.OfflineTranscriber
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Two-run benchmark against a fixed 11-second Russian speech clip
 * (official Sber test sample, MIT licence, bundled as assets/benchmark.wav).
 *
 * Run 1 (cold): model is released before the run so it loads from scratch.
 * Run 2 (warm): same instance still in memory — steady-state speed.
 *
 * The difference between the two numbers shows how much the device pays
 * for a cold model reload (e.g. after Android evicted the process).
 */
class BenchmarkActivity : AppCompatActivity() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    private lateinit var runButton: MaterialButton
    private lateinit var progress: LinearProgressIndicator
    private lateinit var resultsCard: View
    private lateinit var shareRow: View
    private lateinit var rtfHint: View
    private lateinit var heroMultiplierText: MaterialTextView
    private lateinit var providerText: MaterialTextView
    private lateinit var coldTimeText: MaterialTextView
    private lateinit var warmTimeText: MaterialTextView
    private lateinit var longTimeText: MaterialTextView

    private var lastShareText: String = ""

    companion object {
        private const val BENCHMARK_ASSET      = "benchmark.wav"
        private const val BENCHMARK_LONG_ASSET = "benchmark_long.wav"
        private const val AUDIO_DURATION_S     = 11.3f
        private const val LONG_AUDIO_DURATION_S = 71.2f
        private const val WAV_HEADER_BYTES = 44
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        DynamicColors.applyToActivityIfAvailable(this)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_benchmark)

        val toolbar = findViewById<MaterialToolbar>(R.id.topAppBar)
        toolbar.setNavigationOnClickListener { finish() }
        toolbar.setOnMenuItemClickListener { item ->
            if (item.itemId == R.id.action_help) { showHelpDialog(); true } else false
        }

        val scroll = findViewById<View>(R.id.scroll)
        ViewCompat.setOnApplyWindowInsetsListener(scroll) { v, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.navigationBars() or WindowInsetsCompat.Type.displayCutout()
            )
            v.updatePadding(bottom = bars.bottom)
            insets
        }

        runButton           = findViewById(R.id.benchmarkRunButton)
        progress            = findViewById(R.id.benchmarkProgress)
        resultsCard         = findViewById(R.id.benchmarkResultsCard)
        shareRow            = findViewById(R.id.benchmarkShareRow)
        rtfHint             = findViewById(R.id.benchmarkRtfHint)
        heroMultiplierText  = findViewById(R.id.benchmarkHeroMultiplier)
        providerText        = findViewById(R.id.benchmarkProvider)
        coldTimeText   = findViewById(R.id.benchmarkColdTime)
        warmTimeText   = findViewById(R.id.benchmarkWarmTime)
        longTimeText   = findViewById(R.id.benchmarkLongTime)

        findViewById<MaterialButton>(R.id.benchmarkShareButton).setOnClickListener { shareResult() }
        findViewById<MaterialButton>(R.id.benchmarkCopyButton).setOnClickListener { copyResult() }
        runButton.setOnClickListener { runBenchmark() }
    }

    private fun runBenchmark() {
        runButton.isEnabled = false
        progress.visibility = View.VISIBLE
        resultsCard.visibility = View.GONE
        shareRow.visibility = View.GONE
        rtfHint.visibility = View.GONE

        scope.launch {
            try {
                data class BenchResult(
                    val coldMs: Long, val warmMs: Long, val longMs: Long,
                    val provider: String
                )
                val result = withContext(Dispatchers.Default) {
                    val pcm     = loadPcm(BENCHMARK_ASSET)
                    val pcmLong = loadPcm(BENCHMARK_LONG_ASSET)

                    // Force cold start: release current instance so run 1 loads from scratch.
                    OfflineTranscriber.release()

                    // Run 1 — cold (includes model init)
                    val t0 = SystemClock.elapsedRealtime()
                    val transcriber = OfflineTranscriber.getInstance(this@BenchmarkActivity)
                    transcriber.startAudio()
                    transcriber.sendAudioChunk(pcm)
                    transcriber.stopAudioAndGetTranscript()
                    val coldMs = SystemClock.elapsedRealtime() - t0

                    // Run 2 — warm short clip (same instance, already in memory)
                    val t1 = SystemClock.elapsedRealtime()
                    transcriber.startAudio()
                    transcriber.sendAudioChunk(pcm)
                    transcriber.stopAudioAndGetTranscript()
                    val warmMs = SystemClock.elapsedRealtime() - t1

                    // Run 3 — long clip (71 sec, warm model)
                    val t2 = SystemClock.elapsedRealtime()
                    transcriber.startAudio()
                    transcriber.sendAudioChunk(pcmLong)
                    transcriber.stopAudioAndGetTranscript()
                    val longMs = SystemClock.elapsedRealtime() - t2

                    BenchResult(coldMs, warmMs, longMs, transcriber.activeProvider)
                }
                val (coldMs, warmMs, longMs, provider) = result

                val providerLabel = if (provider == "nnapi") "Аппаратный NPU"
                    else "Процессор (NPU недоступен)"
                // Hero = weighted average over warm short + long clips (cold excluded —
                // it's a one-time startup cost, not representative of daily use).
                // Formula: total warm audio duration / total warm processing time → true RTF.
                val totalWarmAudioMs = (AUDIO_DURATION_S + LONG_AUDIO_DURATION_S) * 1000f
                val heroX = totalWarmAudioMs / (warmMs + longMs)
                val device = "${Build.MANUFACTURER} ${Build.MODEL}".trim()

                fun fmtX(x: Float) = if (x >= 10f) "×${x.toInt()}" else "×%.1f".format(x)

                heroMultiplierText.text = fmtX(heroX)
                providerText.text = providerLabel
                coldTimeText.text = "%.1f сек".format(coldMs / 1000f)
                warmTimeText.text = "%.1f сек".format(warmMs / 1000f)
                longTimeText.text = "%.1f сек".format(longMs / 1000f)

                // Dynamic hint: plain-language description of what the numbers mean.
                (rtfHint as? com.google.android.material.textview.MaterialTextView)?.text =
                    "11 секунд аудио: распознано за %.1f сек при первом запуске и за %.1f сек при повторном. ".format(coldMs / 1000f, warmMs / 1000f) +
                    "71 секунда аудио: распознана за %.1f сек.".format(longMs / 1000f)

                val appVersion = packageManager.getPackageInfo(packageName, 0).versionName
                lastShareText = buildString {
                    appendLine("Говорун: голос в текст ($appVersion)")
                    appendLine("$device, Android ${Build.VERSION.RELEASE}, $providerLabel")
                    appendLine("Скорость: ${fmtX(heroX)}")
                    append("11 сек: %.1f / %.1f с, 71 сек: %.1f с".format(coldMs / 1000f, warmMs / 1000f, longMs / 1000f))
                }

                progress.visibility = View.GONE
                resultsCard.visibility = View.VISIBLE
                shareRow.visibility = View.VISIBLE
                rtfHint.visibility = View.VISIBLE
                runButton.isEnabled = true

            } catch (e: Exception) {
                progress.visibility = View.GONE
                runButton.isEnabled = true
            }
        }
    }

    private fun loadPcm(asset: String): ByteArray =
        assets.open(asset).use { stream ->
            val bytes = stream.readBytes()
            bytes.copyOfRange(WAV_HEADER_BYTES, bytes.size)
        }

    private fun shareResult() {
        if (lastShareText.isEmpty()) return
        startActivity(Intent.createChooser(
            Intent(Intent.ACTION_SEND).apply {
                type = "text/plain"
                putExtra(Intent.EXTRA_TEXT, lastShareText)
            }, null
        ))
    }

    private fun copyResult() {
        if (lastShareText.isEmpty()) return
        val cm = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        cm.setPrimaryClip(ClipData.newPlainText("benchmark", lastShareText))
    }

    private fun showHelpDialog() {
        val dialog = MaterialAlertDialogBuilder(this)
            .setTitle(R.string.benchmark_help_title)
            .setMessage(R.string.benchmark_help_body)
            .setPositiveButton(R.string.benchmark_help_close, null)
            .show()
        // Make the GitHub URL in the message body tappable.
        dialog.findViewById<android.widget.TextView>(android.R.id.message)?.apply {
            android.text.util.Linkify.addLinks(this, android.text.util.Linkify.WEB_URLS)
            movementMethod = android.text.method.LinkMovementMethod.getInstance()
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }
}
