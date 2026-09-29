$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Run this validator on Windows.' }
Set-Location (Split-Path $PSScriptRoot -Parent)
$toolBin = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
$env:Path = "$toolBin;$env:Path"
$env:RUSTC = "$toolBin\rustc.exe"
$env:RUSTDOC = "$toolBin\rustdoc.exe"
$env:CARGO_TARGET_DIR = 'target\windows-validation'
foreach ($check in @(
    @{Name='fmt'; Args='fmt --check'},
    @{Name='default'; Args='test --locked -q'},
    @{Name='portable'; Args='test --locked --features portable'},
    @{Name='clippy'; Args='clippy --locked --all-targets -- -D warnings'}
)) {
    Write-Output "START $($check.Name)"
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = "$toolBin\cargo.exe"
    $info.Arguments = $check.Args
    $info.WorkingDirectory = (Get-Location).Path
    $info.UseShellExecute = $false
    $p = [System.Diagnostics.Process]::Start($info)
    $p.WaitForExit()
    Write-Output "END $($check.Name) exit=$($p.ExitCode)"
    if ($p.ExitCode -ne 0) { exit $p.ExitCode }
}
Write-Output 'WINDOWS_VALIDATION_SUCCESS'
