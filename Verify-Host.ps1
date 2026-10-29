param([Parameter(Mandatory=$true)][string]$Component,[string]$Toolchain='')
$ErrorActionPreference='Stop'
$scareComponent=(Resolve-Path -LiteralPath $Component).Path
$scareRoot=Join-Path $PSScriptRoot '../cinnabar'
$scareName='herobrine_check_'+[guid]::NewGuid().ToString('N')
$scareTestRoot=Join-Path $PSScriptRoot ('.local/'+$scareName)
New-Item -ItemType Directory -Force $scareTestRoot | Out-Null
Copy-Item -LiteralPath $scareComponent -Destination (Join-Path $scareTestRoot 'herobrine-watch.wasm')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'tools/host-test.settings.json') -Destination (Join-Path $scareTestRoot 'herobrine-watch.settings.json')
$scareComponent=Join-Path $scareTestRoot 'herobrine-watch.wasm'
$scareExamples=Join-Path $scareRoot 'crates/mod-host/examples'
$scareHadExamples=Test-Path -LiteralPath $scareExamples
New-Item -ItemType Directory -Force $scareExamples | Out-Null
$scareExample=Join-Path $scareExamples ($scareName+'.rs')
$scareSelection=@();if($Toolchain){$scareSelection=@("+$Toolchain")}
try{
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'tools/host-check.rs') -Destination $scareExample
    Push-Location $scareRoot
    try{
        $env:CARGO_BUILD_JOBS='2'
        & cargo @scareSelection run --locked -p mod-host --example $scareName -- $scareComponent
        if($LASTEXITCODE -ne 0){throw 'Herobrine native-host test failed'}
    }finally{Pop-Location}
}finally{
    Remove-Item -LiteralPath $scareExample -ErrorAction SilentlyContinue
    if(-not $scareHadExamples -and (Get-ChildItem -LiteralPath $scareExamples).Count -eq 0){Remove-Item -LiteralPath $scareExamples}
}
