# Publish @dayrecord/mcp to npm
# Prerequisites:
#   1. npm account with access to @dayrecord scope (create org at https://www.npmjs.com/org/create)
#   2. npm login (or NPM_TOKEN with publish permission)
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\publish-mcp.ps1
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\publish-mcp.ps1 -DryRun

param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$pkgDir = Join-Path $root "packages\mcp"

Push-Location $pkgDir
try {
    $whoami = npm whoami 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Not logged in to npm. Run: npm login" -ForegroundColor Yellow
        Write-Host "Scoped package requires org @dayrecord — create at https://www.npmjs.com/org/create"
        exit 1
    }
    Write-Host "npm user: $whoami"

    npm pack --dry-run
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    if ($DryRun) {
        Write-Host "Dry run only — skipping npm publish"
        exit 0
    }

    npm publish --access public
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host ""
    Write-Host "Published. Cursor mcp.json:" -ForegroundColor Green
    Write-Host '  "command": "cmd", "args": ["/c", "npx", "-y", "@dayrecord/mcp"]'
}
finally {
    Pop-Location
}
