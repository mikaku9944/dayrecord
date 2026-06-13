# Probe MCP tools/call and print raw JSON-RPC responses (isError check).
param(
    [string]$Exe = ""
)

if (-not $Exe) {
    $candidates = @(
        "$PSScriptRoot\..\.local-install\bin\dayrecord.exe",
        "$PSScriptRoot\..\target\debug\dayrecord.exe"
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) { $Exe = (Resolve-Path $c).Path; break }
    }
    if (-not $Exe) {
        $found = Get-ChildItem -Path "$env:TEMP\cursor-sandbox-cache" -Recurse -Filter "dayrecord.exe" -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -notmatch '\\deps\\' } |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if ($found) { $Exe = $found.FullName }
    }
}

if (-not (Test-Path $Exe)) {
    Write-Error "dayrecord.exe not found; pass -Exe path"
    exit 1
}

Write-Host "=== probe using: $Exe ===" -ForegroundColor Cyan

$tools = @(
    "get_user_profile",
    "pause_recording",
    "generate_today_summary",
    "get_recording_status"
)

foreach ($tool in $tools) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.Arguments = "mcp"
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $p = [System.Diagnostics.Process]::Start($psi)
    $init = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"1.0"}}}'
    $initialized = '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    $call = "{`"jsonrpc`":`"2.0`",`"id`":2,`"method`":`"tools/call`",`"params`":{`"name`":`"$tool`",`"arguments`":{}}}"

    $p.StandardInput.WriteLine($init)
    $p.StandardInput.WriteLine($initialized)
    $p.StandardInput.WriteLine($call)
    $p.StandardInput.Close()

    if (-not $p.WaitForExit(12000)) {
        $p.Kill()
        Write-Host "`n--- $tool --- TIMEOUT" -ForegroundColor Red
        continue
    }

    $stdout = $p.StandardOutput.ReadToEnd()
    $stderr = $p.StandardError.ReadToEnd()

    Write-Host "`n--- $tool ---" -ForegroundColor Yellow
    $matched = $false
    foreach ($line in ($stdout -split "`n")) {
        if ($line -notmatch '"id"\s*:\s*2') { continue }
        $matched = $true
        try {
            $j = $line | ConvertFrom-Json
            $isError = $j.result.isError
            $structured = $j.result.structuredContent
            Write-Host "isError: $isError"
            if ($structured) {
                Write-Host "structuredContent: $($structured | ConvertTo-Json -Compress -Depth 5)"
            } else {
                $text = $j.result.content[0].text
                Write-Host "content[0].text: $text"
            }
        } catch {
            Write-Host "raw: $line"
        }
    }
    if (-not $matched) {
        Write-Host "no id=2 response" -ForegroundColor Red
        if ($stderr) { Write-Host "stderr: $stderr" }
        if ($stdout) { Write-Host "stdout: $stdout" }
    }
}
