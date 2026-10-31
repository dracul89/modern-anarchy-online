<#
.SYNOPSIS
  A recorded session with the ORIGINAL Anarchy Online client: network log, screenshots and the client's own logs.

.DESCRIPTION
  1. Starts tools/capture_proxy.py on 127.0.0.1:7505 (it follows the hand-offs to the game servers, the shops' and
     instances' servers too), writing captures/<name>-<time>/net.log.
  2. Starts tools/screen_logger.py: a screenshot every 2 s into captures/<name>-<time>/shots (file names are Unix ms, the
     proxy log's clock).
  3. Launches the real install's anarchyonline.exe pointed at the proxy (command-line arguments only: nothing in the
     install changes, and it keeps its original DLLs).
  4. When the client has been closed for 10 s, stops both, copies what the client wrote to its Funcom folder this session
     (stderr.txt, prefs, logs) into captures/<name>-<time>/client and prints a summary.
  The net log holds the account name and the login exchange: captures/ stays out of git; delete it when done with it.

.EXAMPLE
  .\tools\capture_session.ps1 -Name arete
#>
param(
    [string]$Name = 'session',
    [string]$Game = 'C:\Program Files (x86)\Steam\steamapps\common\Anarchy Online',
    [int]$Port = 7505,
    [double]$ShotSeconds = 2
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$dir = Join-Path $repo "captures\$Name-$stamp"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$started = Get-Date

# The port must be free: an old proxy left running would take the connection and log it elsewhere.
$busy = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
if ($busy) { throw "Port $Port is in use by process $($busy[0].OwningProcess): stop it first (an old capture_proxy?)" }
if (Get-Process anarchyonline -ErrorAction SilentlyContinue) { throw 'An Anarchy Online client is already running: close it first' }

$proxy = Start-Process python -ArgumentList @("`"$repo\tools\capture_proxy.py`"", $Port, 'cm.d1.funcom.com:7505', "`"$dir\net.log`"") -WorkingDirectory $repo -WindowStyle Minimized -PassThru
$shots = Start-Process python -ArgumentList @("`"$repo\tools\screen_logger.py`"", "`"$dir\shots`"", $ShotSeconds) -WorkingDirectory $repo -WindowStyle Minimized -PassThru
Start-Sleep -Seconds 2
if ($proxy.HasExited) { throw "The proxy did not start (exit code $($proxy.ExitCode))" }
Write-Host "Recording into $dir"
Write-Host "  proxy pid $($proxy.Id), screenshots pid $($shots.Id)"

# IA = the login server's address as a little-endian integer (16777343 = 127.0.0.1), IP = its port.
$client = Start-Process (Join-Path $Game 'anarchyonline.exe') -WorkingDirectory $Game -ArgumentList "IA16777343 IP$Port DU127.0.0.1:$Port" -PassThru
Write-Host "Original client started (pid $($client.Id)). Play; close the client when done."

# Done when no client has run for 10 s (it may be restarted in between).
$quiet = 0
while ($quiet -lt 10) {
    Start-Sleep -Seconds 1
    if (Get-Process anarchyonline -ErrorAction SilentlyContinue) { $quiet = 0 } else { $quiet++ }
}

New-Item -ItemType File -Force -Path "$dir\shots\stop" | Out-Null
Start-Sleep -Seconds ([math]::Ceiling($ShotSeconds) + 2)
if (-not $shots.HasExited) { Stop-Process -Id $shots.Id -Force }
if (-not $proxy.HasExited) { Stop-Process -Id $proxy.Id -Force }

# What the client wrote this session (its folder is named by a hash of the install's path).
$funcom = Join-Path $env:LOCALAPPDATA 'Funcom\Anarchy Online'
$leaf = Split-Path $Game -Leaf
foreach ($src in Get-ChildItem $funcom -Directory | ForEach-Object { Join-Path $_.FullName $leaf } | Where-Object { Test-Path $_ }) {
    Get-ChildItem $src -Recurse -File | Where-Object { $_.LastWriteTime -ge $started } | ForEach-Object {
        $to = Join-Path "$dir\client" $_.FullName.Substring($src.Length).TrimStart('\')
        New-Item -ItemType Directory -Force -Path (Split-Path $to) | Out-Null
        Copy-Item $_.FullName $to
    }
}

$net = Get-Item "$dir\net.log" -ErrorAction SilentlyContinue
$connections = if ($net) { (Select-String -Path $net.FullName -Pattern ' OPEN ' -SimpleMatch).Count } else { 0 }
$pictures = (Get-ChildItem "$dir\shots" -Filter *.jpg -ErrorAction SilentlyContinue).Count
$files = (Get-ChildItem "$dir\client" -Recurse -File -ErrorAction SilentlyContinue).Count
Write-Host ''
Write-Host "Done: $dir"
Write-Host ("  net.log {0:N0} bytes, {1} connections" -f ($(if ($net) { $net.Length } else { 0 })), $connections)
Write-Host "  $pictures screenshots, $files client files"
