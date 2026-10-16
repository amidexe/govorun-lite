param([switch]$Force)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=Split-Path $PSScriptRoot -Parent
. "$root\..\desktop\scripts\windows-runtime-assets.ps1"
$output=Join-Path $root 'Native\vad'
$valid=-not $Force
foreach($name in $GovorunVadHashes.Keys) {
    $file=Join-Path $output $name
    if(!(Test-Path -LiteralPath $file -PathType Leaf) -or
       (Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant() -cne $GovorunVadHashes[$name]){$valid=$false}
}
if($valid){Write-Output 'PASS pinned VAD runtime already present';return}
$cache=Join-Path $root '.cache'
New-Item -ItemType Directory -Path $cache -Force|Out-Null
$archive=Join-Path $cache ('sherpa-vad-'+[Guid]::NewGuid().ToString('N')+'.nupkg')
$stage=Join-Path $cache ('vad-'+[Guid]::NewGuid().ToString('N'))
try {
    Invoke-WebRequest -UseBasicParsing -Uri 'https://api.nuget.org/v3-flatcontainer/org.k2fsa.sherpa.onnx.runtime.win-x64/1.13.1/org.k2fsa.sherpa.onnx.runtime.win-x64.1.13.1.nupkg' -OutFile $archive
    # Raw signed archive pin; NuGet lock contentHash omits signing metadata.
    if((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -cne 'd65d7f3b755592e6658934b79e193259979036a673c68784437aad917a484ed1'){throw 'Pinned VAD package hash mismatch'}
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    New-Item -ItemType Directory -Path $stage|Out-Null
    $zip=[IO.Compression.ZipFile]::OpenRead($archive)
    try {
        foreach($name in $GovorunVadHashes.Keys) {
            $entry=$zip.GetEntry('runtimes/win-x64/native/'+$name)
            if(!$entry){throw 'Pinned VAD library missing'}
            $file=Join-Path $stage $name
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry,$file)
            if((Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant() -cne $GovorunVadHashes[$name]){throw 'Pinned VAD library hash mismatch'}
        }
    } finally {$zip.Dispose()}
    New-Item -ItemType Directory -Path $output -Force|Out-Null
    foreach($name in $GovorunVadHashes.Keys){Move-Item -LiteralPath (Join-Path $stage $name) -Destination (Join-Path $output $name) -Force}
    Write-Output 'PASS pinned VAD runtime prepared without managed bindings'
} finally {
    if(Test-Path -LiteralPath $archive){Remove-Item -LiteralPath $archive -Force}
    if(Test-Path -LiteralPath $stage){Remove-Item -LiteralPath $stage -Recurse -Force}
}
