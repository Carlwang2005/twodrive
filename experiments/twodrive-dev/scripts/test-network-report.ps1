param(
    [string]$ScriptPath = (Join-Path $PSScriptRoot 'verify-network.ps1'),
    [switch]$LegacyReportDepth
)
$ErrorActionPreference = 'Stop'
# Load only trusted repository function definitions, never the server/restart
# entry point. Network/process operations in Wait-Stage are isolated below.
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($ScriptPath, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -gt 0) { throw 'Verification script syntax errors.' }
$functions = @($ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $true))
foreach ($name in @('Save-Report', 'Wait-Stage', 'Read-NetworkMessages')) {
    $definition = @($functions | Where-Object { $_.Name -eq $name })
    if ($name -eq 'Read-NetworkMessages' -and $LegacyReportDepth -and $definition.Count -eq 0) { continue }
    if ($definition.Count -ne 1) { throw ('Expected exactly one function: ' + $name) }
    $source = $definition[0].Extent.Text
    # Bound the old provider graph to depth 3; do not reproduce an OOM in CI.
    if ($LegacyReportDepth -and $name -eq 'Save-Report') { $source = $source.Replace('-Depth 10', '-Depth 3') }
    . ([scriptblock]::Create($source))
}
function Route-Stamp { 'unchanged-synthetic-route' }
function Get-NetAdapter { [pscustomobject]@{ HardwareInterface = $true; Status = 'Up'; ifIndex = 11 } }
function Get-NetIPAddress {
    [pscustomobject]@{ InterfaceIndex = 11; IPAddress = '192.0.2.2' }
    [pscustomobject]@{ InterfaceIndex = 99; IPAddress = '198.18.0.1' }
}
function Get-NetTCPConnection { }
function Start-Sleep { }
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('td-report-regression-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($temporary) | Out-Null
try {
    $runId = 'td-egress-' + ('0' * 32)
    $controlPath = Join-Path $temporary 'control.json'
    [IO.File]::WriteAllText($controlPath, ('{"run_id":"' + $runId + '","action":"next"}'))
    $logPath = Join-Path $temporary 'synthetic.stderr'
    $expected = @('network: route=physical interface=synthetic fallback=reported-system/proxy')
    [IO.File]::WriteAllLines($logPath, @('non-network diagnostic') + $expected)
    $reportPath = Join-Path $temporary 'report.json'
    $report = [ordered]@{ run_id = $runId; status = 'starting'; stages = [ordered]@{}; cleanup = [ordered]@{} }
    $deadline = (Get-Date).AddSeconds(30)
    $script:current = @{ process = [Diagnostics.Process]::GetCurrentProcess(); stderr = $logPath }
    Wait-Stage 'auto' 'next'
    $json = [IO.File]::ReadAllText($reportPath)
    $decoded = $json | ConvertFrom-Json
    $messages = @($decoded.stages.auto.network_messages)
    if ($messages.Count -ne 1 -or $messages[0] -isnot [string] -or $messages[0] -ne $expected[0]) {
        throw 'Report network_messages must contain only plain strings.'
    }
    if ($json.Length -gt 2048 -or $json -match 'PSDrive|PSProvider|PSPath') { throw 'Report contains provider metadata or exceeds its bounded size.' }
    if ($decoded.status -ne 'auto_ready' -or $decoded.stages.auto.samples -ne 1 -or -not $decoded.stages.auto.proxy_fake_ip_present) { throw 'Stage report lost required values.' }
    if (-not $LegacyReportDepth) {
        if (@(Read-NetworkMessages (Join-Path $temporary 'missing')).Count -ne 0) { throw 'Missing log should have no messages.' }
        [IO.File]::WriteAllLines($logPath, @())
        if (@(Read-NetworkMessages $logPath).Count -ne 0) { throw 'Empty log should have no messages.' }
        [IO.File]::WriteAllLines($logPath, $expected + @('network: route=system/proxy fallback; controlled fixture'))
        if (@(Read-NetworkMessages $logPath).Count -ne 2) { throw 'Multiple network messages were lost.' }
    }
    Write-Host ('Compact stage/report regression passed on PowerShell ' + $PSVersionTable.PSVersion.ToString())
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force
}
