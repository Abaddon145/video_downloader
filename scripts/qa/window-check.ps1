param([int]$ApplicationPid, [string]$Action, [int]$Width = 0, [int]$Height = 0, [long]$WindowHandle = 0)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class DownloadWindow {
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd, int command);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr insertAfter, int x, int y, int width, int height, uint flags);
}
'@
$process = Get-Process -Id $ApplicationPid
$handle = if ($WindowHandle -eq 0) { $process.MainWindowHandle } else { [IntPtr]$WindowHandle }
if ($handle -eq [IntPtr]::Zero) { throw 'Application has no visible main window' }
if ($Action -eq 'close') { [void][DownloadWindow]::SendMessage($handle, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) }
if ($Action -eq 'show') { [void][DownloadWindow]::ShowWindow($handle, 9) }
if ($Action -eq 'resize') {
  $scale = [DownloadWindow]::GetDpiForWindow($handle) / 96.0
  [void][DownloadWindow]::SetWindowPos($handle, [IntPtr]::Zero, 0, 0, [int](($Width + 16) * $scale), [int](($Height + 40) * $scale), 0x16)
}
@{ handle = $handle.ToInt64(); visible = [DownloadWindow]::IsWindowVisible($handle); running = -not $process.HasExited } | ConvertTo-Json -Compress
