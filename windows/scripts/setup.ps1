param(
    [switch]$Force,
    [switch]$InstallVcRuntime
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot -Parent
$modelDirectory = Join-Path $root "Models"
$runtimeDirectory = Join-Path $root "Native\transcribe"
New-Item -ItemType Directory -Force -Path $modelDirectory | Out-Null
New-Item -ItemType Directory -Force -Path $runtimeDirectory | Out-Null

$files = [ordered]@{
    "benchmark.wav" = "https://raw.githubusercontent.com/amidexe/govorun-lite/91444be6316a17a05054cd3f815ae15a02e2dda8/app/src/main/assets/benchmark.wav"
    "benchmark_long.wav" = "https://raw.githubusercontent.com/amidexe/govorun-lite/91444be6316a17a05054cd3f815ae15a02e2dda8/app/src/main/assets/benchmark_long.wav"

    "gigaam-v3-e2e-rnnt-Q8_0.gguf" = "https://huggingface.co/handy-computer/gigaam-v3-e2e-rnnt-gguf/resolve/main/gigaam-v3-e2e-rnnt-Q8_0.gguf"
    "silero_vad.onnx" = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx"
}
$sha256 = @{
    "benchmark.wav" = "d8aaaa18a5098d7c6de0595ae7ac1e64cacd0d4022af3595213bdaf23be77e69"
    "benchmark_long.wav" = "1868ece0195dfa9fc2394be24865d1133c8452a8292a397db45ba8c3ed9e01e3"

    "gigaam-v3-e2e-rnnt-Q8_0.gguf" = "78d63b47723b7f8d78c6113a6ef983b5a86e2a86f6c273e1f5cb6967b1c4467a"
    "silero_vad.onnx" = "9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6"
}

$benchmarkDirectory = Join-Path $root "Assets\Benchmark"
New-Item -ItemType Directory -Force -Path $benchmarkDirectory | Out-Null
foreach ($entry in $files.GetEnumerator()) {
    $targetDirectory = if ($entry.Key -in @("benchmark.wav", "benchmark_long.wav")) { $benchmarkDirectory } else { $modelDirectory }
    $destination = Join-Path $targetDirectory $entry.Key
    $download = $Force -or -not (Test-Path $destination -PathType Leaf)
    if (-not $download) {
        $existingHash = (Get-FileHash $destination -Algorithm SHA256).Hash.ToLowerInvariant()
        $download = $existingHash -ne $sha256[$entry.Key]
    }
    if ($download) {
        Write-Host "Downloading: $($entry.Key)"
        $temporaryDownload = "$destination.download-$([Guid]::NewGuid())"
        try {
            Invoke-WebRequest -Uri $entry.Value -OutFile $temporaryDownload -UseBasicParsing
            if ((Get-Item $temporaryDownload).Length -lt 100) {
                throw "Downloaded model is unexpectedly small: $($entry.Key)"
            }
            $downloadHash = (Get-FileHash $temporaryDownload -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($downloadHash -ne $sha256[$entry.Key]) {
                throw "SHA-256 mismatch for $($entry.Key): $downloadHash"
            }
            Move-Item $temporaryDownload $destination -Force
        }
        finally {
            Remove-Item $temporaryDownload -Force -ErrorAction SilentlyContinue
        }
    } else {
        Write-Host "Already present and verified: $($entry.Key)"
    }
    $actualHash = (Get-FileHash $destination -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $sha256[$entry.Key]) {
        throw "SHA-256 mismatch for $($entry.Key): $actualHash"
    }
}

$legacyModels = @(
    "gigaam_v3_e2e_rnnt_encoder_int8.onnx",
    "gigaam_v3_e2e_rnnt_decoder.onnx",
    "gigaam_v3_e2e_rnnt_joint.onnx",
    "gigaam_v3_e2e_rnnt_tokens.txt"
)
foreach ($name in $legacyModels) {
    $legacyPath = Join-Path $modelDirectory $name
    if (Test-Path $legacyPath -PathType Leaf) { Remove-Item $legacyPath -Force }
}

$runtimeReady = -not $Force
if ($runtimeReady) {
    try { & "$PSScriptRoot\verify-transcribe-runtime.ps1" -Directory $runtimeDirectory }
    catch { $runtimeReady = $false }
}
if (-not $runtimeReady) {
    & "$PSScriptRoot\build-transcribe-runtime.ps1" -OutputDirectory $runtimeDirectory
}

& "$PSScriptRoot\setup-vad-runtime.ps1" -Force:$Force

Write-Host "Model files are ready in $modelDirectory"
Write-Host "ASR runtime is ready in $runtimeDirectory"
if ($InstallVcRuntime) {
    & "$PSScriptRoot\prepare-vc-runtime.ps1" -Install
}
else {
    Write-Host "Visual C++ Runtime installation was not requested."
    Write-Host "Run setup.ps1 -InstallVcRuntime explicitly if the prerequisite is absent."
}
