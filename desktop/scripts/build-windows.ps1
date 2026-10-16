param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$desktop = Join-Path $root 'desktop'
$output = Join-Path $desktop 'release\windows'
if (Test-Path $output) { throw "Move or remove the previous build first: $output" }
foreach ($tool in @('npm.cmd', 'cargo', 'cl')) {
    if (!(Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Required build tool is missing: $tool" }
}
function Invoke-Build([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Build command failed: $Command (exit $LASTEXITCODE)" }
}
. "$PSScriptRoot\windows-runtime-assets.ps1"
. "$PSScriptRoot\windows-crt-assets.ps1"
$previous = @{}
foreach ($key in @('GOVORUN_BUILD_ID', 'GOVORUN_UPDATE_FEED_URL', 'GOVORUN_ACCOUNT_ORIGIN', 'GOVORUN_AUTH_SCHEME', 'TAURI_CONFIG', 'CARGO_TARGET_DIR')) {
    $previous[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
Push-Location $desktop
try {
    $env:GOVORUN_BUILD_ID = [DateTime]::UtcNow.ToString('yyyyMMdd.HHmmZ')
    $env:GOVORUN_UPDATE_FEED_URL = 'https://govorun.club/updates/tauri/test.json'
    $env:GOVORUN_ACCOUNT_ORIGIN = $null
    $env:GOVORUN_AUTH_SCHEME = $null
    $env:TAURI_CONFIG = @{identifier='com.govorun.app'; productName='Говорун'} | ConvertTo-Json -Compress
    if (!$env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $desktop 'target' }
    $gate = $env:GOVORUN_WINDOWS_SDK_GATE
    if (!$gate) { $gate = Join-Path $root 'windows' }
    if (!(Test-Path "$gate\scripts\verify-transcribe-runtime.ps1")) { throw 'Set GOVORUN_WINDOWS_SDK_GATE to the prepared, verified SDK gate' }
    $env:CARGO_TARGET_DIR = [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
    & "$gate\scripts\setup.ps1"
    Invoke-Build 'npm.cmd' @('ci')
    Invoke-Build 'npm.cmd' @('run', 'check')
    Invoke-Build 'npm.cmd' @('run', 'build')
    New-Item -ItemType Directory -Path $output | Out-Null
    New-GovorunSdkAssets -Directory "$output\native" -Gate $gate -VadDirectory "$gate\Native\vad"
    Copy-GovorunAppLocalCrt -Directory $output
    $resources = @{}
    $resources["$output\native"] = 'native/'
    $resources["$desktop\THIRD_PARTY_LICENSES.txt"] = 'THIRD_PARTY_LICENSES.txt'
    Get-ChildItem $output -Filter '*.dll' | ForEach-Object { $resources[$_.FullName] = $_.Name }
    $key = (Get-Content "$desktop\src-tauri\update-public-keys.json" -Raw | ConvertFrom-Json).ordinary
    $config = @{identifier='com.govorun.app';productName='Говорун';
        plugins=@{updater=@{pubkey=$key}};
        bundle=@{targets=@('nsis');createUpdaterArtifacts=$false;resources=$resources}}
    $configPath = Join-Path $output 'local-tauri-bundle.json'
    $config | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $configPath
    Invoke-Build 'npm.cmd' @('exec', '--', 'tauri', 'build', '--ci', '--features', 'custom-protocol', '--config', $configPath)
    Write-Host "PASS local build: $output"
} finally {
    Pop-Location
    foreach ($key in $previous.Keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key], 'Process') }
}
