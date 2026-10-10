param(
    [Parameter(Mandatory=$true)][string]$ReferenceDirectory,
    [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-f]{40}$')][string]$ReferenceRevision,
    [Parameter(Mandatory=$true)][string]$SourceDirectory,
    [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceRevision,
    [string]$Python = "$env:USERPROFILE\AppData\Local\Programs\Python\Python313\python.exe",
    [switch]$Avx2
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Run this validator on Windows.' }
$root = Split-Path $PSScriptRoot -Parent
$reference = (Resolve-Path $ReferenceDirectory).Path
$sourceDirectoryPath = (Resolve-Path $SourceDirectory).Path
$toolBin = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
$env:Path = "$toolBin;$env:Path"
$env:RUSTC = "$toolBin\rustc.exe"
$env:RUSTDOC = "$toolBin\rustdoc.exe"
$env:CARGO_TARGET_DIR = Join-Path $root 'target\windows-validation'

function Invoke-FrontendCheck([string]$Executable, [string[]]$CheckArguments) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $Executable
    $info.Arguments = ($CheckArguments | ForEach-Object { '"{0}"' -f $_ }) -join ' '
    $info.WorkingDirectory = $root
    $info.UseShellExecute = $false
    $process = [System.Diagnostics.Process]::Start($info)
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "Frontend check failed with exit $($process.ExitCode)." }
}

# Explicit revision and file hashes identify the actual supplied reference.
# This runner does not fetch libraries or enable GitHub CI.
$libraries = @('j.dll')
if ($Avx2) { $libraries += 'javx2.dll' }
foreach ($library in $libraries) {
    if (-not (Test-Path (Join-Path $reference $library))) { throw "Missing C reference: $library" }
}
Invoke-FrontendCheck "$toolBin\cargo.exe" @('build', '--locked', '--bins', '--examples')
Invoke-FrontendCheck $Python @('-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_*.py')
foreach ($library in $libraries) {
    $env:J_LIBRARY = Join-Path $reference $library
    $variant = if ($library -eq 'j.dll') { 'j64' } else { 'avx2' }
    foreach ($mode in @('direct', 'semantic-reference', 'parser-capture')) {
        Write-Output "START $variant $mode"
        $checkArguments = @('tools/conformance.py', '--binary', 'target/windows-validation/debug/rustj.exe',
            '--reference-revision', $ReferenceRevision, '--report', "reports/frontend-$variant-$mode-windows.json")
        if ($mode -eq 'semantic-reference') { $checkArguments += '--semantic-reference' }
        if ($mode -eq 'parser-capture') {
            $checkArguments[2] = 'target/windows-validation/debug/examples/capture_probe.exe'
            $checkArguments += '--parser-capture'
        }
        Invoke-FrontendCheck $Python $checkArguments
    }
    Write-Output "START $variant stages"
    Invoke-FrontendCheck $Python @('tools/frontend_stage_conformance.py',
        '--binary', 'target/windows-validation/debug/examples/frontend_probe.exe',
        '--source-directory', $sourceDirectoryPath, '--source-revision', $SourceRevision,
        '--reference-revision', $ReferenceRevision, '--report', "reports/frontend-$variant-stages-windows.json")
    Write-Output "START $variant words"
    Invoke-FrontendCheck $Python @('tools/word_conformance.py', '--binary', 'target/windows-validation/debug/examples/scan_words.exe',
        '--reference-revision', $ReferenceRevision, '--report', "reports/frontend-$variant-words-windows.json")
    Write-Output "START $variant vocabulary"
    Invoke-FrontendCheck $Python @('tools/vocabulary_audit.py',
        '--binary', 'target/windows-validation/debug/examples/frontend_probe.exe',
        '--source-directory', $sourceDirectoryPath, '--source-revision', $SourceRevision,
        '--reference-revision', $ReferenceRevision, '--report', "reports/vocabulary-$variant-windows.json")
    Write-Output "START $variant spelling"
    Invoke-FrontendCheck $Python @('tools/spelling_conformance.py',
        '--binary', 'target/windows-validation/debug/examples/frontend_probe.exe',
        '--source-directory', $sourceDirectoryPath, '--source-revision', $SourceRevision,
        '--reference-revision', $ReferenceRevision, '--report', "reports/spelling-$variant-windows.json")
    Write-Output "START $variant name-syntax"
    Invoke-FrontendCheck $Python @('tools/name_syntax_conformance.py',
        '--binary', 'target/windows-validation/debug/examples/frontend_probe.exe',
        '--source-directory', $sourceDirectoryPath, '--source-revision', $SourceRevision,
        '--reference-revision', $ReferenceRevision, '--report', "reports/name-syntax-$variant-windows.json")
    Write-Output "START $variant numeric-syntax"
    Invoke-FrontendCheck $Python @('tools/numeric_syntax_conformance.py',
        '--binary', 'target/windows-validation/debug/examples/frontend_probe.exe',
        '--source-directory', $sourceDirectoryPath, '--source-revision', $SourceRevision,
        '--reference-revision', $ReferenceRevision, '--report', "reports/numeric-syntax-$variant-windows.json")
    Write-Output "START $variant scan-contract"
    Invoke-FrontendCheck $Python @('tools/scan_contract_conformance.py',
        '--binary', 'target/windows-validation/debug/examples/scan_contract_probe.exe',
        '--source-directory', $sourceDirectoryPath, '--source-revision', $SourceRevision,
        '--reference-revision', $ReferenceRevision, '--report', "reports/scan-$variant-contract-windows.json")
}
Write-Output 'WINDOWS_FRONTEND_VALIDATION_SUCCESS'
