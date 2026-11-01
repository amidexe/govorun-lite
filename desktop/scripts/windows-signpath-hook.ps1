param(
    [Parameter(Mandatory=$true)][ValidateSet('Capture','Import')][string]$Mode,
    [Parameter(Mandatory=$true)][string]$File,
    [Parameter(Mandatory=$true)][string]$UnsignedDirectory,
    [Parameter(Mandatory=$true)][string]$Version,
    [string]$Application,
    [string]$SignedDirectory,
    [string]$Python='python.exe'
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
# Called only by NSIS !uninstfinalize; no certificates, credentials or trust-store changes.
if($Version -cnotmatch '^\d+\.\d+\.\d+$'){throw 'Expected a release version'}
$product=-join (0x0413,0x043e,0x0432,0x043e,0x0440,0x0443,0x043d|ForEach-Object{[char]$_})
function Assert-PlainFile([string]$Path) {
    $f=Get-Item -LiteralPath $Path -Force
    if($f.PSIsContainer -or ($f.Attributes -band [IO.FileAttributes]::ReparsePoint) -or $f.Length -gt 512MB){throw 'Expected a bounded plain file'}
}
function Assert-Metadata([string]$Path) {
    Assert-PlainFile $Path
    $v=[Diagnostics.FileVersionInfo]::GetVersionInfo([IO.Path]::GetFullPath($Path))
    if($v.ProductName -cne $product -or $v.ProductVersion -cne $Version){throw 'Unexpected product metadata'}
}
Assert-Metadata $File
$unsigned=Join-Path $UnsignedDirectory 'uninstall.exe'
if($Mode -eq 'Capture') {
    if(Test-Path -LiteralPath $UnsignedDirectory){throw 'Capture directory must be new'}
    Assert-Metadata $Application
    foreach($path in @($File,$Application)) {
        if((Get-AuthenticodeSignature -LiteralPath $path).Status -ne 'NotSigned'){throw 'Capture requires unsigned files'}
    }
    New-Item -ItemType Directory -Path $UnsignedDirectory|Out-Null
    Copy-Item -LiteralPath $File -Destination $unsigned
    # Tauri has already applied its NSIS bundle marker at this point.
    Copy-Item -LiteralPath $Application -Destination (Join-Path $UnsignedDirectory 'Govorun.exe')
} else {
    Assert-Metadata $unsigned
    if((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash -cne (Get-FileHash -LiteralPath $unsigned -Algorithm SHA256).Hash){throw 'Regenerated uninstaller differs from the signing input'}
    $signed=Join-Path $SignedDirectory 'uninstall.exe'
    Assert-Metadata $signed
    & $Python (Join-Path $PSScriptRoot 'windows-signing-pair.py') $unsigned $signed
    if($LASTEXITCODE -ne 0){throw 'Signed uninstaller payload differs'}
    Copy-Item -LiteralPath $signed -Destination $File -Force
    if((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash -cne (Get-FileHash -LiteralPath $signed -Algorithm SHA256).Hash){throw 'Signed uninstaller replacement failed'}
}
Write-Output "PASS NSIS uninstaller $Mode"
