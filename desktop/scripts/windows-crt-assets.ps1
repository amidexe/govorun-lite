# Ship the Visual Studio redistributable DLLs next to the executable. Never
# install a machine-wide runtime from a per-user installer.
$GovorunCrtNames=@('concrt140.dll','msvcp140.dll','msvcp140_1.dll','msvcp140_2.dll',
    'msvcp140_atomic_wait.dll','msvcp140_codecvt_ids.dll','vccorlib140.dll',
    'vcruntime140.dll','vcruntime140_1.dll','vcruntime140_threads.dll')

function Assert-GovorunAppLocalCrt {
    param([string]$Directory)
    foreach($name in $GovorunCrtNames) {
        $path=Join-Path $Directory $name
        if(!(Test-Path -LiteralPath $path -PathType Leaf)){throw "App-local CRT missing: $name"}
        $file=Get-Item -LiteralPath $path
        if($file.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'CRT must be a regular file'}
        $version=$file.VersionInfo
        if([Version]::new($version.FileMajorPart,$version.FileMinorPart,$version.FileBuildPart,$version.FilePrivatePart) -lt [Version]'14.44.35211.0') {
            throw "App-local CRT too old or invalid: $name"
        }
        $signature=Get-AuthenticodeSignature -LiteralPath $path
        if($signature.Status -ne 'Valid' -or !$signature.SignerCertificate -or
           $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') {
            throw "App-local CRT must retain its valid Microsoft signature: $name"
        }
        $reader=[IO.BinaryReader]::new([IO.File]::OpenRead($path))
        try {
            if($reader.ReadUInt16() -ne 0x5a4d){throw 'Invalid CRT PE header'}
            $reader.BaseStream.Position=0x3c
            $offset=$reader.ReadInt32()
            if($offset -lt 64 -or $offset -gt $reader.BaseStream.Length-6){throw 'Invalid CRT PE offset'}
            $reader.BaseStream.Position=$offset
            if($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664){throw 'CRT must be Windows x64'}
        } finally {$reader.Dispose()}
    }
}

function Copy-GovorunAppLocalCrt {
    param([string]$Directory,[string]$RedistDirectory=$env:VCToolsRedistDir)
    if(!$RedistDirectory){throw 'Visual Studio redistributable source is required'}
    $source=Join-Path $RedistDirectory 'x64\Microsoft.VC143.CRT'
    Assert-GovorunAppLocalCrt -Directory $source
    foreach($name in $GovorunCrtNames) {
        $destination=Join-Path $Directory $name
        if(Test-Path -LiteralPath $destination){throw 'CRT output must not already exist'}
    }
    foreach($name in $GovorunCrtNames){Copy-Item -LiteralPath (Join-Path $source $name) -Destination $Directory}
    Assert-GovorunAppLocalCrt -Directory $Directory
}
