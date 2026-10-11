param(
    [Parameter(Mandatory=$true)][string]$State,
    [Parameter(Mandatory=$true)][string]$ExpectedSha256,
    [Parameter(Mandatory=$true)][string]$Owner,
    [ValidateRange(120,7200)][int]$Lifetime=7200
)
# One bootstrap: the original paired serve stays running throughout.
$ErrorActionPreference='Stop'
$bundle=(Resolve-Path -LiteralPath $PSScriptRoot).ProviderPath
if ((Split-Path $bundle -Leaf) -notmatch '^td-control-[a-f0-9]{32}$') { throw 'Unexpected control bundle name.' }
$statePath=(Resolve-Path -LiteralPath $State).ProviderPath
if ($statePath.StartsWith($bundle + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Private state must be outside the shared control directory.' }
$source=Join-Path $bundle 'twodrive-dev.exe'
if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne $ExpectedSha256) { throw 'Agent executable SHA256 mismatch.' }
$preserved=@{}
foreach ($name in @('identity.json','control.json')) { $preserved[$name]=(Get-FileHash -LiteralPath (Join-Path $statePath $name) -Algorithm SHA256).Hash }
$runtime=Join-Path ([IO.Path]::GetTempPath()) (Split-Path $bundle -Leaf)
if (Test-Path -LiteralPath $runtime) { throw 'Agent runtime already exists; refusing overwrite.' }
$previousTemp=$env:TEMP
$previousTmp=$env:TMP
$result=[ordered]@{ agent_exit_code=$null; identity_and_grants_preserved=$false; bootstrap_runtime_removed=$false; executable_payload_removed=$false }
try {
    New-Item -ItemType Directory -Path $runtime | Out-Null
    $exe=Join-Path $runtime 'twodrive-dev.exe'
    Copy-Item -LiteralPath $source -Destination $exe
    if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $ExpectedSha256) { throw 'Private executable copy differs from expected SHA256.' }
    # All child state/sample files remain inside this owned runtime.
    $env:TEMP=$runtime
    $env:TMP=$runtime
    & $exe --state $statePath test-agent --queue $bundle --owner $Owner --lifetime $Lifetime
    $result.agent_exit_code=$LASTEXITCODE
} finally {
    $env:TEMP=$previousTemp
    $env:TMP=$previousTmp
    $same=$true
    foreach ($name in $preserved.Keys) { if ((Get-FileHash -LiteralPath (Join-Path $statePath $name) -Algorithm SHA256).Hash -ne $preserved[$name]) { $same=$false } }
    $result.identity_and_grants_preserved=$same
    Remove-Item -LiteralPath $runtime -Recurse -Force
    $result.bootstrap_runtime_removed=-not (Test-Path -LiteralPath $runtime)
    Remove-Item -LiteralPath $source -Force
    $result.executable_payload_removed=-not (Test-Path -LiteralPath $source)
    # The Linux controller collects this small receipt and removes the bundle.
    [IO.File]::WriteAllText((Join-Path $bundle 'bootstrap-cleanup.json'), ($result | ConvertTo-Json -Compress))
}
