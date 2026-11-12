<#
.SYNOPSIS
  Builds the release client and puts together a folder (and a zip) to play from anywhere: dist/ModernAnarchyOnline.

.DESCRIPTION
  The folder holds mao-client.exe, the bundled skins (assets/, which the client looks for beside itself), README.txt
  (tools/package-readme.txt), the repository's LICENSE files when it has any, and play.bat, which
  finds the game (the folder with cd_image: a path given to it, else the usual Steam and Funcom places) and starts the
  launcher. The client keeps its settings, hotbars, crash.log and the like in %APPDATA%\ModernAnarchyOnline (or
  mao.ini's folder when one sits where it runs). Nothing from the game is copied: the player needs Anarchy Online installed.

  On Linux and macOS, tools/package.sh does the same (mao-client, assets/ and a play.sh that looks in the Steam, Proton,
  Wine and CrossOver places); see docs/PLATFORMS.md.

  With -Version the zip is named for a release: ModernAnarchyOnline-<version>-windows-x64.zip (the release workflow,
  .github/workflows/release.yml, calls it so; see docs/RELEASING.md).

.EXAMPLE
  .\tools\package.ps1
.EXAMPLE
  .\tools\package.ps1 -Version 0.2.0
#>
param(
    [string]$Out = 'dist',
    [string]$Version = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    # (cargo's warnings go to stderr, which Windows PowerShell would take as an error)
    $ErrorActionPreference = 'Continue'
    cargo build --release --locked -p mao-client 2>&1 | Out-Host
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -ne 0) { throw 'the build failed' }
    $dir = Join-Path $root "$Out/ModernAnarchyOnline"
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    New-Item -ItemType Directory -Force $dir | Out-Null
    Copy-Item target/release/mao-client.exe $dir
    Copy-Item -Recurse assets $dir
    Copy-Item tools/package-readme.txt (Join-Path $dir 'README.txt')
    Get-ChildItem -Path $root -File -Filter 'LICENSE*' | Copy-Item -Destination $dir
    $bat = @'
@echo off
rem Starts Modern Anarchy Online: the launcher (log in, pick a character), then the game.
rem The game folder (the one with cd_image) is the first argument, else the first of the usual places that has it.
setlocal
set "GAME=%~1"
if "%GAME%"=="" for %%G in (
  "C:\Program Files (x86)\Steam\steamapps\common\Anarchy Online"
  "C:\Program Files\Steam\steamapps\common\Anarchy Online"
  "C:\Funcom\Anarchy Online"
  "C:\Program Files (x86)\Funcom\Anarchy Online"
) do if not defined GAME if exist "%%~G\cd_image" set "GAME=%%~G"
if "%GAME%"=="" (
  echo Anarchy Online was not found. Start this with its folder, for example:
  echo   play.bat "D:\Games\Anarchy Online"
  pause
  exit /b 1
)
"%~dp0mao-client.exe" "%GAME%" play
if errorlevel 1 (
  echo The client stopped with an error; see crash.log in %APPDATA%\ModernAnarchyOnline.
  pause
)
'@
    Set-Content -Encoding ascii -Path (Join-Path $dir 'play.bat') -Value $bat
    $name = if ($Version) { "ModernAnarchyOnline-$Version-windows-x64" } else { 'ModernAnarchyOnline' }
    $zip = Join-Path $root "$Out/$name.zip"
    if (Test-Path $zip) { Remove-Item $zip }
    Compress-Archive -Path $dir -DestinationPath $zip
    $size = '{0:N1} MB' -f ((Get-Item $zip).Length / 1MB)
    Write-Host "packaged: $dir ($zip, $size)"
} finally {
    Pop-Location
}
