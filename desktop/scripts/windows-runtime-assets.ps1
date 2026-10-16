# Pinned native recognition assets only. No C# executable or managed runtime.
$GovorunVadHashes=@{
    'onnxruntime.dll'='756c938b0f2d14c45c7ad469c6429041d7e905ee6c6ad6049aa67ce937567e97'
    'sherpa-onnx-c-api.dll'='e772bd7c03fc7d352d453b9a7033b553e971e0aa03153a36e52e5f4b45871cc5'
}

function Assert-GovorunSdkAssets {
    param([string]$Directory,[string]$Gate)
    foreach($name in $GovorunVadHashes.Keys) {
        $file=Join-Path $Directory $name
        if(!(Test-Path -LiteralPath $file -PathType Leaf) -or
           (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -cne $GovorunVadHashes[$name]) {
            throw "Pinned VAD runtime missing or changed: $name"
        }
    }
    & "$Gate\scripts\verify-transcribe-runtime.ps1" -Directory $Directory
    $retired=@(Get-ChildItem $Directory -Recurse -File|Where-Object {
        $_.Extension -eq '.exe' -or $_.Name -match '\.runtimeconfig\.json$' -or
        $_.Name -in @('coreclr.dll','hostfxr.dll','System.Private.CoreLib.dll','Govorun.dll')
    })
    if($retired.Count){throw 'Retired C# executable or managed runtime in SDK assets'}
    foreach($model in @(
        @{Name='gigaam-v3-e2e-rnnt-Q8_0.gguf';Size=273724832;Hash='78d63b47723b7f8d78c6113a6ef983b5a86e2a86f6c273e1f5cb6967b1c4467a'},
        @{Name='silero_vad.onnx';Size=643854;Hash='9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6'}
    )) {
        $files=@(Get-ChildItem $Directory -Recurse -File -Filter $model.Name)
        $expected=Join-Path $Directory ('Models\'+$model.Name)
        if($files.Count -ne 1 -or $files[0].FullName -cne $expected){throw 'Package model must occur exactly once at Models'}
        if($files[0].Length -ne $model.Size){throw 'Package recognition model size changed'}
        if((Get-FileHash $expected -Algorithm SHA256).Hash.ToLowerInvariant() -cne $model.Hash){throw 'Package recognition model hash changed'}
    }
    if(@(Get-ChildItem "$Directory\Models" -Force).Count -ne 2){throw 'Unexpected recognition model files'}
    foreach($source in @((Join-Path $Gate 'THIRD_PARTY_LICENSES.txt'))+@(Get-ChildItem "$Gate\LICENSES" -File|ForEach-Object {$_.FullName})) {
        $relative=$source.Substring($Gate.Length+1)
        $packaged=Join-Path $Directory $relative
        if(!(Test-Path -LiteralPath $packaged -PathType Leaf) -or
           (Get-FileHash $source -Algorithm SHA256).Hash -cne (Get-FileHash $packaged -Algorithm SHA256).Hash){throw 'Packaged third-party notice missing or changed'}
    }
}

function New-GovorunSdkAssets {
    param([string]$Directory,[string]$Gate,[string]$VadDirectory,[scriptblock]$ValidateExtra)
    if(Test-Path -LiteralPath $Directory){throw 'Runtime asset output must be new'}
    $runtime=Join-Path $Gate 'Native\transcribe'
    & "$Gate\scripts\verify-transcribe-runtime.ps1" -Directory $runtime
    $manifest=Get-Content "$runtime\govorun-build.json" -Raw|ConvertFrom-Json
    if(@($manifest.files).Count -gt 64){throw 'Runtime manifest bound'}
    $names=@($manifest.files|ForEach-Object {$_.name})
    if(@($names|Sort-Object -Unique).Count -ne $names.Count -or
       @($names|Where-Object {$_ -cnotmatch '^[a-zA-Z0-9][a-zA-Z0-9_.-]*\.(dll|json)$' -or
           $_ -match '^(Govorun|coreclr|hostfxr|System\.)' -or $_ -match '\.runtimeconfig\.json$'}).Count -ne 0){
        throw 'Invalid runtime manifest file name'
    }
    $stage=$Directory+'.stage-'+[Guid]::NewGuid().ToString('N')
    try {
        New-Item -ItemType Directory -Path $stage|Out-Null
        foreach($name in $names){Copy-Item -LiteralPath (Join-Path $runtime $name) -Destination $stage}
        Copy-Item -LiteralPath "$runtime\govorun-build.json" -Destination $stage
        foreach($name in $GovorunVadHashes.Keys){Copy-Item -LiteralPath (Join-Path $VadDirectory $name) -Destination $stage}
        Copy-Item -LiteralPath "$Gate\Models" -Destination "$stage\Models" -Recurse
        Copy-Item -LiteralPath "$Gate\THIRD_PARTY_LICENSES.txt" -Destination $stage
        Copy-Item -LiteralPath "$Gate\LICENSES" -Destination "$stage\LICENSES" -Recurse
        Assert-GovorunSdkAssets -Directory $stage -Gate $Gate
        if($ValidateExtra){& $ValidateExtra $stage}
        Move-Item -LiteralPath $stage -Destination $Directory
    } finally {
        if(Test-Path -LiteralPath $stage){Remove-Item -LiteralPath $stage -Recurse -Force}
    }
}
