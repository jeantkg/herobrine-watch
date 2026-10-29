param([string]$Toolchain='')
$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try{
    & python tools/prepare_skin.py
    if($LASTEXITCODE -ne 0){throw 'Skin validation failed'}
    $env:CARGO_BUILD_JOBS='2'
    $scareSelection=@();if($Toolchain){$scareSelection=@("+$Toolchain")}
    & cargo @scareSelection test --locked
    if($LASTEXITCODE -ne 0){throw 'Tests failed'}
    & cargo @scareSelection build --locked --release --target wasm32-unknown-unknown
    if($LASTEXITCODE -ne 0){throw 'WASM build failed'}
    New-Item -ItemType Directory -Force dist | Out-Null
    & cargo @scareSelection run --locked --example pack -- target/wasm32-unknown-unknown/release/herobrine_watch_mod.wasm dist/herobrine-watch.wasm
    if($LASTEXITCODE -ne 0){throw 'Component validation failed'}
    Copy-Item default.settings.json dist/herobrine-watch.settings.json
    Copy-Item README.md dist/README.md
    Copy-Item LICENSE dist/LICENSE
    $scareVersion=(Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
    Compress-Archive -LiteralPath @('dist/herobrine-watch.wasm','dist/herobrine-watch.settings.json','dist/README.md','dist/LICENSE') -DestinationPath "dist/herobrine-watch-$scareVersion.zip" -Force
}finally{Pop-Location}
