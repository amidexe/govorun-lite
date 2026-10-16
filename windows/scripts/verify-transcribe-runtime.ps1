param([Parameter(Mandatory = $true)][string]$Directory)

$ErrorActionPreference = 'Stop'
$manifestPath = Join-Path $Directory 'govorun-build.json'
if (-not (Test-Path $manifestPath -PathType Leaf)) { throw 'Optimized ASR runtime manifest is missing.' }
$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
if ($manifest.recipe -ne 'transcribe-0.2.1-openblas-0.3.34-v1' -or
    $manifest.sourceSha256 -ne '9da9128a6b9c10aaad20d982dd9d19ac13f9a3561f34ccad8555803c74c503b3' -or
    $manifest.backendArchiveSha256 -ne '241bd33754c6816af4ddfaa34d7f8781977d2f1854515b57843a3640edff492d' -or
    $manifest.blasArchiveSha256 -ne 'e9cb6134541f36c27346d5fc5995652f060fba227cebbbabcbda5a5a44d7c76b') {
    throw 'ASR runtime build provenance does not match the pinned recipe.'
}
$required = @('transcribe.dll', 'libopenblas.dll', 'ggml.dll', 'ggml-base.dll', 'ggml-vulkan.dll',
    'ggml-cpu-alderlake.dll', 'ggml-cpu-cannonlake.dll', 'ggml-cpu-cascadelake.dll',
    'ggml-cpu-haswell.dll', 'ggml-cpu-icelake.dll', 'ggml-cpu-sandybridge.dll',
    'ggml-cpu-skylakex.dll', 'ggml-cpu-sse42.dll', 'ggml-cpu-x64.dll', 'contract.json')
foreach ($name in $required) {
    $entry = @($manifest.files | Where-Object { $_.name -ceq $name })
    $path = Join-Path $Directory $name
    if ($entry.Count -ne 1 -or -not (Test-Path $path -PathType Leaf)) { throw "ASR runtime file is missing: $name" }
    if ((Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry[0].sha256) {
        throw "ASR runtime file changed since its build: $name"
    }
}
Write-Host 'PASS pinned ASR runtime and file hashes'
