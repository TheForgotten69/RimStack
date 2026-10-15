# Builds target\packages\RimStack-<version>-windows-x86_64.zip, and the
# installer beside it where Inno Setup's ISCC is on PATH: rimstack.exe
# built with the `dist` profile, with Qt's libraries, plugins and QML
# modules and the MSVC runtime beside it, put there by windeployqt.
#
# Run from a "x64 Native Tools" prompt, with Qt's bin folder on PATH or
# QT_BIN set to it. The MSVC target is the only one: the DDS encoder
# ships its Windows code for MSVC alone. SIGN_CERT (a .pfx) and
# SIGN_PASSWORD sign the program and the installer; without a signature
# SmartScreen warns on every download.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..\..")

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq "rimstack-app").version
$target = "x86_64-pc-windows-msvc"

cargo build --profile dist -p rimstack-app --target $target
if ($LASTEXITCODE) { exit $LASTEXITCODE }

$stage = "target\packaging\RimStack"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory $stage | Out-Null
Copy-Item "target\$target\dist\rimstack.exe" $stage

$deploy = if ($env:QT_BIN) { Join-Path $env:QT_BIN "windeployqt.exe" } else { "windeployqt" }
& $deploy --release --qmldir crates\app\qml --no-translations --compiler-runtime "$stage\rimstack.exe"
if ($LASTEXITCODE) { exit $LASTEXITCODE }

function Sign($file) {
    if (-not $env:SIGN_CERT) { return }
    signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /f $env:SIGN_CERT /p $env:SIGN_PASSWORD $file
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
}
Sign "$stage\rimstack.exe"

New-Item -ItemType Directory target\packages -Force | Out-Null
$zip = "target\packages\RimStack-$version-windows-x86_64.zip"
Compress-Archive -Path "$stage\*" -DestinationPath $zip -Force
Write-Output $zip

if (Get-Command ISCC -ErrorAction SilentlyContinue) {
    ISCC /Qp "/DVersion=$version" "/DStage=$((Resolve-Path $stage).Path)" "/Otarget\packages" packaging\windows\rimstack.iss
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
    $setup = "target\packages\RimStack-$version-setup.exe"
    Sign $setup
    Write-Output $setup
}
