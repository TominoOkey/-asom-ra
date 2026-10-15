$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root

$Version = (Select-String -Path 'Cargo.toml' -Pattern '^version = "([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value
if (-not $Version) { throw 'Nelze zjistit verzi z Cargo.toml.' }

rustup target add x86_64-pc-windows-msvc
$PreviousRustFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = (($PreviousRustFlags + ' -C target-feature=+crt-static').Trim())
    cargo build --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Cargo build skončil s chybou $LASTEXITCODE." }
}
finally {
    $env:RUSTFLAGS = $PreviousRustFlags
}

$Makensis = (Get-Command 'makensis.exe' -ErrorAction SilentlyContinue).Source
if (-not $Makensis) {
    $Candidate = Join-Path ${env:ProgramFiles(x86)} 'NSIS/makensis.exe'
    if (Test-Path $Candidate) { $Makensis = $Candidate }
}
if (-not $Makensis) { throw 'Chybí NSIS. Nainstaluj NSIS a spusť build znovu.' }

New-Item -ItemType Directory -Force -Path (Join-Path $Root 'dist/windows') | Out-Null
$PortableExe = Join-Path $Root "dist/windows/Casomira-$Version-windows-x64.exe"
Copy-Item -Force (Join-Path $Root 'target/x86_64-pc-windows-msvc/release/casomira.exe') $PortableExe
Copy-Item -Force (Join-Path $PSScriptRoot 'windows/README.txt') (Join-Path $Root 'dist/windows/README.txt')
$PortableZip = Join-Path $Root "dist/windows/Casomira-$Version-windows-x64.zip"
Remove-Item -Force $PortableZip -ErrorAction SilentlyContinue
Compress-Archive -Path $PortableExe, (Join-Path $Root 'dist/windows/README.txt') -DestinationPath $PortableZip
& $Makensis -V2 "/DAppVersion=$Version" (Join-Path $PSScriptRoot 'windows/Casomira.nsi')
if ($LASTEXITCODE -ne 0) { throw "NSIS skončil s chybou $LASTEXITCODE." }
Write-Host "Hotovo: dist/windows/Casomira-Setup-$Version.exe a Casomira-$Version-windows-x64.exe"
