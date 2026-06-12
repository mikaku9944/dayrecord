# Hot-reload dev: Vite on :1420 + Tauri loading devUrl (rebuilds with empty embedded assets).
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$frontend = Join-Path $root "frontend"

$vite = Start-Process -FilePath "npm" -ArgumentList "run", "dev" -WorkingDirectory $frontend -PassThru
Start-Sleep -Seconds 3

$env:TAURI_CONFIG = '{"build":{"devUrl":"http://localhost:1420"}}'
try {
    Push-Location $root
    cargo run -p dayrecord-app
} finally {
    if ($vite -and -not $vite.HasExited) {
        Stop-Process -Id $vite.Id -Force -ErrorAction SilentlyContinue
    }
    Remove-Item Env:TAURI_CONFIG -ErrorAction SilentlyContinue
}
