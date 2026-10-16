param(
    [string]$OutputDirectory = '',
    [string]$CacheDirectory = '',
    [string]$VcVarsPath = ''
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$project = Split-Path $PSScriptRoot -Parent
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $project 'Native\transcribe' }
if (-not $CacheDirectory) { $CacheDirectory = Join-Path ([IO.Path]::GetTempPath()) 'govorun-asr-downloads' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force $CacheDirectory | Out-Null

# Rebuild only the host library: the signed-off ggml CPU/Vulkan backends keep
# their original ABI and CPU dispatch, including the SSE and Haswell variants.
$sourceHash = '9da9128a6b9c10aaad20d982dd9d19ac13f9a3561f34ccad8555803c74c503b3'
$runtimeHash = '241bd33754c6816af4ddfaa34d7f8781977d2f1854515b57843a3640edff492d'
$blasHash = 'e9cb6134541f36c27346d5fc5995652f060fba227cebbbabcbda5a5a44d7c76b'
function Fetch([string]$Url, [string]$Name, [string]$Hash) {
    $path = Join-Path $CacheDirectory $Name
    if (-not (Test-Path $path -PathType Leaf) -or (Get-FileHash $path).Hash.ToLowerInvariant() -ne $Hash) {
        $temporary = "$path.download-$([Guid]::NewGuid())"
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $temporary
            if ((Get-FileHash $temporary).Hash.ToLowerInvariant() -ne $Hash) { throw "Download SHA-256 mismatch: $Name" }
            Move-Item $temporary $path -Force
        } finally { Remove-Item $temporary -Force -ErrorAction SilentlyContinue }
    }
    return $path
}
$sourceArchive = Fetch 'https://codeload.github.com/handy-computer/transcribe.cpp/tar.gz/refs/tags/v0.2.1' 'transcribe-0.2.1-source.tar.gz' $sourceHash
$runtimeArchive = Fetch 'https://github.com/handy-computer/transcribe.cpp/releases/download/v0.2.1/transcribe-native-0.2.1-windows-x86_64-cpu-vulkan.tar.gz' 'transcribe-0.2.1-runtime.tar.gz' $runtimeHash
$blasArchive = Fetch 'https://github.com/OpenMathLib/OpenBLAS/releases/download/v0.3.34/OpenBLAS-0.3.34-x64.zip' 'openblas-0.3.34-x64.zip' $blasHash
$cmakeArchive = Fetch 'https://github.com/Kitware/CMake/releases/download/v3.31.8/cmake-3.31.8-windows-x86_64.zip' 'cmake-3.31.8-x64.zip' '81aa9964dbabd71fe02e7ec50472fd3ad56138c49944515ece9001efbff8d719'
if (-not $VcVarsPath) {
    $candidates = @('C:\BuildTools\VisualStudio\VC\Auxiliary\Build\vcvars64.bat')
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $install = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($install) { $candidates += Join-Path $install 'VC\Auxiliary\Build\vcvars64.bat' }
    }
    $VcVarsPath = $candidates | Where-Object { Test-Path $_ -PathType Leaf } | Select-Object -First 1
}
if (-not $VcVarsPath -or -not (Test-Path $VcVarsPath -PathType Leaf)) { throw 'Visual Studio x64 C++ build tools are required.' }
$environmentBefore = @{}
Get-ChildItem Env: | ForEach-Object { $environmentBefore[$_.Name] = $_.Value }
try {
$values = & cmd.exe /d /c ('"' + $VcVarsPath + '" >nul && set')
if ($LASTEXITCODE -ne 0) { throw 'Visual C++ environment initialization failed.' }
foreach ($value in $values) {
    if ($value -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') }
}
$work = Join-Path $CacheDirectory ('build-' + [Guid]::NewGuid())
New-Item -ItemType Directory $work | Out-Null
try {
    Expand-Archive -LiteralPath $cmakeArchive -DestinationPath $work
    Expand-Archive -LiteralPath $blasArchive -DestinationPath "$work\openblas"
    New-Item -ItemType Directory "$work\runtime" | Out-Null
    tar -xzf $sourceArchive -C $work
    if ($LASTEXITCODE -ne 0) { throw 'ASR source extraction failed.' }
    tar -xzf $runtimeArchive -C "$work\runtime"
    if ($LASTEXITCODE -ne 0) { throw 'ASR backend extraction failed.' }
    $runtime = (Get-ChildItem "$work\runtime" -Recurse -Filter transcribe.dll -File | Select-Object -First 1).DirectoryName
    foreach ($name in @('ggml', 'ggml-base')) {
        $exports = & dumpbin.exe /nologo /exports "$runtime\$name.dll"
        if ($LASTEXITCODE -ne 0) { throw 'ASR DLL export inspection failed.' }
        $lines = @("LIBRARY $name.dll", 'EXPORTS')
        foreach ($line in $exports) {
            if ($line -match '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(\S+)') { $lines += $matches[1] }
        }
        if ($lines.Count -lt 10) { throw "Invalid export table: $name" }
        [IO.File]::WriteAllLines("$runtime\$name.def", $lines, [Text.Encoding]::ASCII)
        & lib.exe /nologo /machine:x64 "/def:$runtime\$name.def" "/out:$runtime\$name.lib"
        if ($LASTEXITCODE -ne 0) { throw 'ASR import library creation failed.' }
    }
    $cmake = "$work\cmake-3.31.8-windows-x86_64\bin\cmake.exe"
    & $cmake -S "$project\BuildSupport\Transcribe" -B "$work\build" -G 'NMake Makefiles' -DCMAKE_BUILD_TYPE=Release "-DTRANSCRIBE_SOURCE=$work/transcribe.cpp-0.2.1" "-DOPENBLAS_ROOT=$work/openblas" "-DRUNTIME_ROOT=$runtime"
    if ($LASTEXITCODE -ne 0) { throw 'Optimized ASR configuration failed.' }
    & $cmake --build "$work\build" --config Release
    if ($LASTEXITCODE -ne 0) { throw 'Optimized ASR compilation failed.' }
    $dependencies = & dumpbin.exe /nologo /imports "$work\build\bin\transcribe.dll"
    if ($LASTEXITCODE -ne 0 -or ($dependencies -join "`n") -notmatch '(?ms)^\s+libopenblas\.dll\s*\n(?<imports>.*?)(?=^\s+\S+\.dll\s*$|\z)') {
        throw 'ASR decoder does not link the pinned OpenBLAS library.'
    }
    $blasImports = $matches.imports
    $blasExports = & dumpbin.exe /nologo /exports "$work\openblas\bin\libopenblas.dll"
    if ($LASTEXITCODE -ne 0) { throw 'OpenBLAS export inspection failed.' }
    foreach ($symbol in @('cblas_sgemv', 'cblas_sgemm')) {
        # The official MinGW import library binds by ordinal, not by name.
        if (($blasExports -join "`n") -notmatch "(?m)^\s+(\d+)\s+[0-9A-F]+\s+[0-9A-F]+\s+$symbol\s*$") {
            throw "OpenBLAS is missing $symbol."
        }
        $ordinal = $matches[1]
        if ($blasImports -notmatch "(?m)^\s+Ordinal\s+$ordinal\s*$" -and $blasImports -notmatch "\b$symbol\b") {
            throw "ASR decoder does not import $symbol."
        }
    }
    $stage = "$OutputDirectory.stage-$([Guid]::NewGuid())"
    New-Item -ItemType Directory -Force $stage | Out-Null
    try {
        Get-ChildItem $runtime -File | Where-Object { $_.Extension -eq '.dll' -or $_.Name -eq 'contract.json' } | Copy-Item -Destination $stage
        Copy-Item "$work\build\bin\transcribe.dll" $stage -Force
        Copy-Item "$work\openblas\bin\libopenblas.dll" $stage
        $files = @(Get-ChildItem $stage -File | Sort-Object Name | ForEach-Object {
            @{ name = $_.Name; sha256 = (Get-FileHash $_.FullName).Hash.ToLowerInvariant() }
        })
        @{ recipe = 'transcribe-0.2.1-openblas-0.3.34-v1'; sourceSha256 = $sourceHash;
            backendArchiveSha256 = $runtimeHash; blasArchiveSha256 = $blasHash; files = $files } |
            ConvertTo-Json -Depth 5 | Set-Content "$stage\govorun-build.json" -Encoding UTF8
        & "$PSScriptRoot\verify-transcribe-runtime.ps1" -Directory $stage
        $backup = "$OutputDirectory.previous-$([Guid]::NewGuid())"
        if (Test-Path $OutputDirectory) { Move-Item $OutputDirectory $backup }
        try { Move-Item $stage $OutputDirectory }
        catch { if (Test-Path $backup) { Move-Item $backup $OutputDirectory }; throw }
        if (Test-Path $backup) { Remove-Item $backup -Recurse -Force }
    } finally { if (Test-Path $stage) { Remove-Item $stage -Recurse -Force } }
} finally { Remove-Item $work -Recurse -Force }
} finally {
    # vcvars sets Platform=x64. Leaking it changes the downstream .NET output
    # directory and breaks the normal AnyCPU packaging/verification contract.
    Get-ChildItem Env: | Where-Object { -not $environmentBefore.ContainsKey($_.Name) } |
        ForEach-Object { [Environment]::SetEnvironmentVariable($_.Name, $null, 'Process') }
    foreach ($entry in $environmentBefore.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process')
    }
}
Write-Host "PASS optimized ASR runtime: $OutputDirectory"
