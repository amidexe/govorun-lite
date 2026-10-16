param(
    [string]$OutputFile = "",
    [switch]$Install
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Test-VcRuntimeInstalled {
    $runtime = Get-ItemProperty `
        "HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" `
        -ErrorAction SilentlyContinue
    if ($null -eq $runtime -or $runtime.Installed -ne 1) { return $false }
    $versionText = ([string]$runtime.Version).TrimStart("v")
    try {
        return [Version]$versionText -ge [Version]"14.44.35211.0"
    }
    catch {
        return $false
    }
}

function Assert-MicrosoftSignature([string]$Path) {
    $signature = Get-AuthenticodeSignature $Path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Microsoft Visual C++ Redistributable has an invalid signature: $($signature.Status)."
    }
    if ($null -eq $signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -notmatch "Microsoft Corporation") {
        throw "Visual C++ Redistributable is not signed by Microsoft Corporation."
    }
}

$temporary = [string]::IsNullOrWhiteSpace($OutputFile)
if ($Install -and (Test-VcRuntimeInstalled)) {
    Write-Host "Microsoft Visual C++ Runtime is already installed."
    exit 0
}
if ($temporary) {
    $OutputFile = Join-Path ([IO.Path]::GetTempPath()) `
        ("govorun-vc-redist-" + [Guid]::NewGuid() + ".exe")
}

try {
    if (-not (Test-Path $OutputFile -PathType Leaf)) {
        $parent = Split-Path $OutputFile -Parent
        if (-not [string]::IsNullOrWhiteSpace($parent)) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
        $download = "$OutputFile.download-$([Guid]::NewGuid())"
        try {
            Write-Host "Downloading Microsoft Visual C++ 2015-2022 Redistributable..."
            Invoke-WebRequest `
                -Uri "https://aka.ms/vc14/vc_redist.x64.exe" `
                -OutFile $download `
                -UseBasicParsing
            Assert-MicrosoftSignature $download
            Move-Item $download $OutputFile -Force
        }
        finally {
            Remove-Item $download -Force -ErrorAction SilentlyContinue
        }
    }

    Assert-MicrosoftSignature $OutputFile
    if ($Install -and -not (Test-VcRuntimeInstalled)) {
        Write-Host "Installing Microsoft Visual C++ Runtime..."
        $process = Start-Process `
            -FilePath $OutputFile `
            -ArgumentList "/install", "/quiet", "/norestart" `
            -Wait `
            -PassThru
        if ($process.ExitCode -notin @(0, 1638, 3010)) {
            throw "Visual C++ Redistributable installer failed with exit code $($process.ExitCode)."
        }
        if (-not (Test-VcRuntimeInstalled)) {
            throw "Visual C++ Runtime installation did not register the x64 runtime."
        }
    }

    if (-not $Install) {
        Write-Host "Verified Microsoft prerequisite: $OutputFile"
    }
}
finally {
    if ($temporary) {
        Remove-Item $OutputFile -Force -ErrorAction SilentlyContinue
    }
}
