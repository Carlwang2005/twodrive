param([string]$ScriptPath=(Join-Path $PSScriptRoot 'start-test-agent.ps1'))
$ErrorActionPreference='Stop'
$directory=Join-Path ([IO.Path]::GetTempPath()) ('td-control-path-' + [guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path $directory | Out-Null
    # Rust canonicalize on Windows returns a verbatim path.
    $State='\\?\' + $directory
    $resolved=Resolve-Path -LiteralPath $State
    if ([IO.Directory]::Exists($resolved.Path)) { throw 'Expected provider-qualified legacy path to be rejected by native IO.' }
    if (-not [IO.Directory]::Exists($resolved.ProviderPath)) { throw 'ProviderPath must preserve a native Windows path.' }
    # Execute the exact path-assignment statement used by the bootstrap.
    $tokens=$null; $errors=$null
    $ast=[Management.Automation.Language.Parser]::ParseFile($ScriptPath,[ref]$tokens,[ref]$errors)
    if ($errors.Count) { throw 'Bootstrap parse failed.' }
    $assignment=$ast.Find({param($node) $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$statePath'}, $true)
    if (-not $assignment) { throw 'Missing bootstrap state assignment.' }
    . ([ScriptBlock]::Create($assignment.Extent.Text))
    if (-not [IO.Directory]::Exists($statePath)) { throw 'Bootstrap state path is not usable by native code.' }
    Write-Host 'PASS: exact bootstrap path assignment accepts Rust verbatim state; legacy Path fails native IO.'
} finally { Remove-Item -LiteralPath $directory -Recurse -Force -ErrorAction SilentlyContinue }
