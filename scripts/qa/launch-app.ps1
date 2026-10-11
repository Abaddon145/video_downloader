param([string]$Executable, [string]$DataDirectory, [int]$Port = 9224)
$ErrorActionPreference = 'Stop'
$env:VIDEO_DOWNLOADER_DATA_DIR = $DataDirectory
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $DataDirectory 'webview2'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$process = Start-Process -FilePath $Executable -WindowStyle Hidden -PassThru
$process.Id
