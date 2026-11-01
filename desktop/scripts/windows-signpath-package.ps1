param(
    [ValidateSet('Functions','Repack')][string]$Mode='Functions',
    [string]$SignedDirectory,
    [string]$Python='python.exe'
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$GovorunPackagingRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent

function Write-GovorunNsisSigningHook([string]$Path,[string]$Phase,[string]$Variant,[string]$Version,[string]$SignedRoot='') {
    if($Phase -notin @('Capture','Import') -or $Variant -notin @('full','update')){throw 'Invalid signing phase'}
    $root=$GovorunPackagingRoot
    # A Windows PowerShell child of pwsh inherits incompatible module paths.
    # Keep NSIS callbacks in the same PowerShell installation as the build.
    $shell=Join-Path $PSHOME $(if($PSVersionTable.PSEdition -eq 'Core'){'pwsh.exe'}else{'powershell.exe'})
    if(!(Test-Path -LiteralPath $shell -PathType Leaf)){throw 'Packaging PowerShell host missing'}
    $capture="$root\desktop\release\windows\external-signing\capture\$Variant"
    $arguments="-Mode $Phase -File `"%1`" -UnsignedDirectory `"$capture`" -Version $Version"
    if($Phase -eq 'Capture'){$arguments+=" -Application `"$root\desktop\target\release\Govorun.exe`""}
    else{$arguments+=" -SignedDirectory `"$SignedRoot\$Variant`" -Python `"$Python`""}
    $body="!include `"$root\desktop\src-tauri\installer\cleanup.nsh`"`r`n"+
        "!uninstfinalize '`"$shell`" -NoProfile -ExecutionPolicy Bypass -File `"$PSScriptRoot\windows-signpath-hook.ps1`" $arguments' = 0`r`n"
    [IO.File]::WriteAllText($Path,$body,[Text.UTF8Encoding]::new($true))
}

function Save-GovorunNsisRecipe([string]$Variant) {
    $root=$GovorunPackagingRoot
    $source="$root\desktop\target\release\nsis\x64"
    $destination="$root\desktop\release\windows\external-signing\recipes\$Variant"
    if(Test-Path -LiteralPath $destination){throw 'Recipe output must be new'}
    New-Item -ItemType Directory -Path $destination|Out-Null
    foreach($f in Get-ChildItem -LiteralPath $source -File){
        if($f.Extension -in @('.nsi','.nsh')){Copy-Item -LiteralPath $f.FullName -Destination $destination}
    }
    if(!(Test-Path "$destination\installer.nsi")){throw 'Rendered NSIS recipe missing'}
}

function Assert-GovorunNoBuildPaths([string]$Path) {
    $bytes=[IO.File]::ReadAllBytes($Path)
    foreach($encoding in @([Text.Encoding]::UTF8,[Text.Encoding]::Unicode)){
        $text=$encoding.GetString($bytes)
        foreach($prefix in @($GovorunPackagingRoot,$env:USERPROFILE)){
            if(!$prefix){continue}
            foreach($form in @($prefix,$prefix.Replace('\','/'))){
                if($text.IndexOf($form,[StringComparison]::OrdinalIgnoreCase) -ge 0){throw 'Original build path remains in signing input'}
            }
        }
    }
}

function Complete-GovorunExternalSigningPreparation([string]$Version) {
    $root=$GovorunPackagingRoot
    $stage="$root\desktop\release\windows\external-signing"
    $full="$stage\capture\full\Govorun.exe"
    $update="$stage\capture\update\Govorun.exe"
    if((Get-FileHash $full -Algorithm SHA256).Hash -cne (Get-FileHash $update -Algorithm SHA256).Hash){throw 'Full and update application payloads differ'}
    $input="$stage\signing-input"
    New-Item -ItemType Directory -Path "$input\full","$input\update"|Out-Null
    Copy-Item -LiteralPath $full -Destination "$input\Govorun.exe"
    foreach($variant in @('full','update')){Copy-Item -LiteralPath "$stage\capture\$variant\uninstall.exe" -Destination "$input\$variant\uninstall.exe"}
    Remove-Item -LiteralPath $full,$update
    foreach($f in Get-ChildItem -LiteralPath $input -Recurse -File){Assert-GovorunNoBuildPaths $f.FullName}
    # Only public build assets and rendered NSIS input; no compiler caches or credentials.
    $files=@(Get-ChildItem "$root\desktop\release\windows","$root\desktop\target\.tauri\NSIS" -Recurse -File)
    $inventory=@(foreach($f in $files|Sort-Object FullName){
        if($f.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Build input link forbidden'}
        @{path=$f.FullName.Substring($root.Length+1).Replace('\','/');size=$f.Length;sha256=(Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
    if($inventory.Count -gt 10000){throw 'Too many packaging inputs'}
    $manifest=@{schemaVersion=1;workspace=$root;version=$Version;files=$inventory;product='Govorun';productionReady=$false}
    [IO.File]::WriteAllText("$stage\packaging-inputs.json",($manifest|ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
    Write-Output 'PASS full/update NSIS signing inputs prepared'
}

function Invoke-GovorunExternalSigningRepack([string]$SignedRoot) {
    $root=$GovorunPackagingRoot
    $stage="$root\desktop\release\windows\external-signing"
    $manifest=Get-Content -LiteralPath "$stage\packaging-inputs.json" -Raw -Encoding UTF8|ConvertFrom-Json
    if($manifest.schemaVersion -ne 1 -or $manifest.workspace -cne $root -or $manifest.version -cnotmatch '^\d+\.\d+\.\d+$'){throw 'Packaging workspace or version mismatch'}
    if(@($manifest.files).Count -gt 10000){throw 'Too many packaging inputs'}
    $declared=@($manifest.files|ForEach-Object{$_.path}|Sort-Object)
    if(@($declared|Sort-Object -Unique).Count -ne $declared.Count){throw 'Duplicate packaging input'}
    $actualInputs=@(Get-ChildItem "$root\desktop\release\windows","$root\desktop\target\.tauri\NSIS" -Recurse -Force -File|
        Where-Object {$_.FullName -cne "$stage\packaging-inputs.json"}|
        ForEach-Object {$_.FullName.Substring($root.Length+1).Replace('\','/')}|Sort-Object)
    if(($actualInputs -join '|') -cne ($declared -join '|')){throw 'Packaging inventory does not cover the exact input file set'}
    foreach($entry in $manifest.files){
        if($entry.path -cnotmatch '^desktop/(release/windows/|target/\.tauri/NSIS/)' -or $entry.path -match '(^|/)\.\.?(/|$)' -or $entry.path -match '[:\\]'){throw 'Packaging input outside its boundary'}
        $path=Join-Path $root $entry.path
        $f=Get-Item -LiteralPath $path -Force
        if($f.PSIsContainer -or ($f.Attributes -band [IO.FileAttributes]::ReparsePoint) -or $f.Length -ne $entry.size -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.sha256){throw 'Packaging input changed'}
    }
    $expected=@('Govorun.exe','full/uninstall.exe','update/uninstall.exe')
    $actual=@(Get-ChildItem -LiteralPath $SignedRoot -Recurse -File|ForEach-Object{$_.FullName.Substring(([IO.Path]::GetFullPath($SignedRoot)).TrimEnd('\').Length+1).Replace('\','/')}|Sort-Object)
    if(($actual -join '|') -cne (($expected|Sort-Object) -join '|')){throw 'Unexpected signed inner file set'}
    foreach($name in $expected){
        & $Python (Join-Path $PSScriptRoot 'windows-signing-pair.py') (Join-Path "$stage\signing-input" $name) (Join-Path $SignedRoot $name)
        if($LASTEXITCODE -ne 0){throw 'Signed inner payload changed'}
    }
    $binary="$root\desktop\target\release\Govorun.exe"
    New-Item -ItemType Directory -Path (Split-Path $binary -Parent) -Force|Out-Null
    Copy-Item -LiteralPath (Join-Path $SignedRoot 'Govorun.exe') -Destination $binary
    $compiler="$root\desktop\target\.tauri\NSIS\makensis.exe"
    $outer="$stage\installer-signing-input"
    if(Test-Path -LiteralPath $outer){throw 'Installer signing output must be new'}
    foreach($variant in @('full','update')){
        $recipe="$stage\recipes\$variant"
        $working="$root\desktop\target\release\nsis\x64"
        New-Item -ItemType Directory -Path $working -Force|Out-Null
        Copy-Item -LiteralPath "$recipe\installer.nsi" -Destination $working
        Get-ChildItem -LiteralPath $recipe -Filter '*.nsh' -File|ForEach-Object{Copy-Item -LiteralPath $_.FullName -Destination $working}
        Write-GovorunNsisSigningHook "$stage\$variant-hook.nsh" 'Import' $variant $manifest.version $SignedRoot
        Push-Location $working
        try {
            & $compiler '-INPUTCHARSET' 'UTF8' '-OUTPUTCHARSET' 'UTF8' '-V2' 'installer.nsi'
            if($LASTEXITCODE -ne 0){throw 'NSIS signed repack failed'}
            New-Item -ItemType Directory -Path "$outer\$variant"|Out-Null
            Copy-Item -LiteralPath '.\nsis-output.exe' -Destination "$outer\$variant\Setup.exe"
        } finally {Pop-Location}
        Assert-GovorunNoBuildPaths "$outer\$variant\Setup.exe"
    }
    Write-Output 'PASS full/update installers repacked; outer signature and release admission still pending'
}

if($Mode -eq 'Repack'){Invoke-GovorunExternalSigningRepack $SignedDirectory}
