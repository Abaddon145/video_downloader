param([Parameter(Mandatory=$true)][string]$Archive, [Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference = 'Stop'
if ([IO.Path]::GetExtension($Archive) -eq '.7z') {
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    $taskArchiveTool = Join-Path $env:SystemRoot 'System32/tar.exe'
    & $taskArchiveTool -xf $Archive -C $Destination
    if ($LASTEXITCODE -ne 0) { throw '组件解压失败' }
} else {
    Expand-Archive -LiteralPath $Archive -DestinationPath $Destination -Force
}
