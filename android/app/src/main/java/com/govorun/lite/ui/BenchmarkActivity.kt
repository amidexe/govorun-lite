package com.govorun.lite.ui

import android.app.ActivityManager
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.graphics.Bitmap
import android.os.Build
import android.os.Bundle
import android.os.SystemClock
import android.view.WindowManager
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.ui.benchmark.BenchmarkImageExport
import com.govorun.lite.ui.benchmark.BenchmarkScreen
import com.govorun.lite.ui.benchmark.BenchmarkUiState
import com.govorun.lite.ui.design.GovorunTheme
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.Locale
import java.security.MessageDigest
import com.govorun.lite.BuildConfig
import com.govorun.lite.transcriber.RecognitionSessionCoordinator
import com.govorun.lite.diagnostics.LifecycleDiagnosticReason
import com.govorun.lite.stylization.LocalDiagnosticJournal
import com.govorun.lite.ui.benchmark.RecognitionDiagnosticReport

class BenchmarkActivity : ComponentActivity() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var state by mutableStateOf(BenchmarkUiState())
    private var shareText = ""
    private var summaryText = ""
    private var checkJob: Job? = null
    private var reportState = "idle"
    private var reportStage = "fixtures"
    private lateinit var systemReport: String

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        systemReport = systemReport()
        refreshReport()
        setContent {
            GovorunTheme {
                BenchmarkScreen(
                    state = state,
                    onBack = ::finish,
                    onRun = ::runBenchmark,
                    onShare = ::shareResultImage,
                    onSave = ::saveResultImage,
                    onCopy = ::copyReport,
                    onCancel = { checkJob?.cancel() },
                )
            }
        }
    }

    private fun runBenchmark() {
        if (state.running) return
        val coordinator = RecognitionSessionCoordinator.process
        val lease = coordinator.acquireDiagnostic()
        if (lease == null) {
            state = state.copy(error = "Сначала завершите диктовку")
            return
        }
        shareText = ""; summaryText = ""; reportState = "running"; reportStage = "fixtures"
        state = BenchmarkUiState(running = true)
        refreshReport()
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        checkJob = scope.launch {
            try {
                val result = withContext(Dispatchers.Default) { executeBenchmark() }
                reportState = "done"; showResult(result)
                LocalDiagnosticJournal.record(this@BenchmarkActivity, LifecycleDiagnosticReason.CHECK_COMPLETED)
            } catch (_: CancellationException) {
                reportState = "cancelled"; state = state.copy(running = false, error = "Проверка отменена")
            } catch (error: Throwable) {
                if (error !is Exception && error !is LinkageError) throw error
                reportState = "failed"
                val reason = when (reportStage) {
                    "fixtures" -> LifecycleDiagnosticReason.CHECK_SAMPLES_FAILED
                    "model" -> LifecycleDiagnosticReason.CHECK_MODEL_FAILED
                    else -> LifecycleDiagnosticReason.CHECK_RECOGNITION_FAILED
                }
                LocalDiagnosticJournal.record(this@BenchmarkActivity, reason)
                state = state.copy(running = false, error = "Проверка не завершена. Подробности — в отчёте.")
            } finally {
                coordinator.release(lease)
                refreshReport()
                window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }.also { job -> job.invokeOnCompletion { coordinator.release(lease) } }
    }

    private suspend fun progress(stage: String) {
        currentCoroutineContext().ensureActive()
        withContext(Dispatchers.Main) { reportStage = stage; state = state.copy(stage = stage); refreshReport() }
    }

    private suspend fun executeBenchmark(): RawResult {
        progress("fixtures")
        val pcm = loadPcm("benchmark.wav")
        val pcmLong = loadPcm("benchmark_long.wav")
        progress("model")
        val startedCold = SystemClock.elapsedRealtime()
        val transcriber = OfflineTranscriber.createDiagnostic(this)
        try {
            val loadMs = SystemClock.elapsedRealtime() - startedCold
            progress("cold")
            check(transcriber.transcribe(pcm).isNotBlank())
            val coldMs = SystemClock.elapsedRealtime() - startedCold
            progress("warm")
            val startedWarm = SystemClock.elapsedRealtime()
            check(transcriber.transcribe(pcm).isNotBlank())
            val warmMs = SystemClock.elapsedRealtime() - startedWarm
            progress("long")
            val startedLong = SystemClock.elapsedRealtime()
            check(transcriber.transcribe(pcmLong).isNotBlank())
            return RawResult(coldMs, warmMs, SystemClock.elapsedRealtime() - startedLong,
                transcriber.activeProvider, loadMs)
        } finally { transcriber.closeDiagnostic() }
    }

    private fun systemReport(): String {
        val memory = ActivityManager.MemoryInfo()
        getSystemService(ActivityManager::class.java).getMemoryInfo(memory)
        val cpu = if (Build.VERSION.SDK_INT >= 31) Build.SOC_MODEL else Build.SUPPORTED_ABIS.firstOrNull() ?: "не определён"
        return "Говорун ${BuildConfig.VERSION_NAME} (${BuildConfig.VERSION_CODE}) · ${BuildConfig.BUILD_ID}\n" +
            "${deviceName()} · Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})\n" +
            "CPU: $cpu · доступно потоков: ${Runtime.getRuntime().availableProcessors()}\n" +
            "Память: ${memory.totalMem / 1_048_576} MiB\nДрайвер CPU: система Android; GPU в этом тесте не проверяется"
    }

    private fun refreshReport() {
        state = state.copy(report = RecognitionDiagnosticReport.render(systemReport, reportState, reportStage, shareText, summaryText))
    }

    private fun copyReport() {
        getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Диагностика Говоруна", state.report))
        Toast.makeText(this, "Отчёт скопирован", Toast.LENGTH_SHORT).show()
    }

    private fun showResult(result: RawResult) {
        val provider = if (result.provider == "nnapi") "NNAPI" else "CPU · Процессор"
        val speed = (82_540f / (result.warmMs + result.longMs))
        val speedText = if (speed >= 10f) "×${speed.toInt()}" else "×%.1f".format(speed)
        val rating = when {
            speed >= 20f -> "Превосходный результат"
            speed >= 13f -> "Отличный результат"
            speed >= 8f -> "Хороший результат"
            else -> null
        }
        val device = deviceName()
        val system = "Android ${Build.VERSION.RELEASE}"
        state = BenchmarkUiState(
            speed = speedText,
            rating = rating,
            provider = provider,
            coldTime = formatDuration(result.coldMs),
            warmTime = formatDuration(result.warmMs),
            longTime = formatDuration(result.longMs),
            device = device,
            system = system,
        )
        val version = packageManager.getPackageInfo(packageName, 0).versionName
        summaryText = RecognitionDiagnosticReport.summary(
            version.orEmpty(), device, if (result.provider == "nnapi") "NNAPI" else "CPU", speedText.replace('×', 'x'),
        )
        shareText = buildString {
            appendLine("Говорун: голос в текст ($version)")
            appendLine("${Build.MANUFACTURER} ${Build.MODEL}, Android ${Build.VERSION.RELEASE}, $provider")
            appendLine("Скорость: ${speedText.replace('×', 'x')}") // cp1251 forums turn «×» into «Ч»
            appendLine("11 сек: ${result.coldMs} / ${result.warmMs} мс, 71 сек: ${result.longMs} мс")
            appendLine("Загрузка модели: ${result.loadMs} мс (входит в первый запуск)")
            append("Фактический ускоритель каждого из трёх замеров: $provider")
        }
    }

    private fun loadPcm(asset: String): ByteArray = assets.open(asset).use {
        val bytes = it.readBytes()
        val expected = when (asset) {
            "benchmark.wav" -> "d8aaaa18a5098d7c6de0595ae7ac1e64cacd0d4022af3595213bdaf23be77e69"
            "benchmark_long.wav" -> "1868ece0195dfa9fc2394be24865d1133c8452a8292a397db45ba8c3ed9e01e3"
            else -> error("Unsupported fixture")
        }
        val hash = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { byte -> "%02x".format(byte) }
        check(hash == expected)
        bytes.copyOfRange(44, bytes.size)
    }

    private suspend fun shareResultImage(bitmap: Bitmap) {
        val intent = withContext(Dispatchers.IO) {
            BenchmarkImageExport(this@BenchmarkActivity).shareIntent(bitmap, shareText)
        }
        startActivity(Intent.createChooser(intent, null))
    }

    private suspend fun saveResultImage(bitmap: Bitmap) {
        withContext(Dispatchers.IO) {
            BenchmarkImageExport(this@BenchmarkActivity).saveToGallery(bitmap)
        }
        Toast.makeText(this, "Изображение сохранено в галерею", Toast.LENGTH_SHORT).show()
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private fun deviceName(): String {
        val manufacturer = Build.MANUFACTURER.replaceFirstChar {
            if (it.isLowerCase()) it.titlecase(Locale.getDefault()) else it.toString()
        }
        return if (Build.MODEL.startsWith(manufacturer, ignoreCase = true)) {
            Build.MODEL
        } else {
            "$manufacturer ${Build.MODEL}"
        }
    }
}

internal fun formatDuration(milliseconds: Long): String {
    if (milliseconds < 1_000) return "$milliseconds мс"
    val seconds = milliseconds / 1_000.0
    val formatted = String.format(Locale.forLanguageTag("ru"), "%.2f", seconds)
        .trimEnd('0')
        .trimEnd(',', '.')
    return "$formatted с"
}

private data class RawResult(
    val coldMs: Long,
    val warmMs: Long,
    val longMs: Long,
    val provider: String,
    val loadMs: Long,
)
