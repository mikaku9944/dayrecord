# Generates minimal placeholder icons for Tauri bundle/CI.
$ErrorActionPreference = "Stop"
$iconDir = Join-Path $PSScriptRoot "..\crates\dayrecord-app\icons"
New-Item -ItemType Directory -Force -Path $iconDir | Out-Null

Add-Type -AssemblyName System.Drawing

function Write-PngIcon {
    param([string]$Path, [int]$Size = 32)
    $bmp = New-Object System.Drawing.Bitmap $Size, $Size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.Color]::FromArgb(255, 37, 99, 235))
    $g.Dispose()
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Write-IcoFile {
    param([string]$Path, [int]$Size = 32)
    $bmp = New-Object System.Drawing.Bitmap $Size, $Size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.Color]::FromArgb(255, 37, 99, 235))
    $g.Dispose()
    $fs = [System.IO.File]::Create($Path)
    ([System.Drawing.Icon]::FromHandle($bmp.GetHicon())).Save($fs)
    $fs.Close()
    $bmp.Dispose()
}

Write-PngIcon (Join-Path $iconDir "32x32.png") 32
Write-PngIcon (Join-Path $iconDir "128x128.png") 128
Write-PngIcon (Join-Path $iconDir "128x128@2x.png") 256
Write-PngIcon (Join-Path $iconDir "icon.png") 256
Write-IcoFile (Join-Path $iconDir "icon.ico") 32
Copy-Item (Join-Path $iconDir "icon.png") (Join-Path $iconDir "icon.icns") -Force

Write-Host "Icons written to $iconDir"
