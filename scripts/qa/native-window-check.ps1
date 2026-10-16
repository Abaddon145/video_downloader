param([int]$ApplicationPid,[string]$Screenshot='', [switch]$Freeze, [switch]$StopEngine, [long]$WindowHandle=0)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class NativeVideoCheck {
 public delegate bool EnumProc(IntPtr h, IntPtr p);
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int Left,Top,Right,Bottom;}
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,EnumProc f,IntPtr p);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder s,int n);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr GetProp(IntPtr h,string name);
 [DllImport("ntdll.dll")] public static extern int NtSuspendProcess(IntPtr h);
 public static List<object> Children(IntPtr h){var result=new List<object>();EnumChildWindows(h,(w,p)=>{uint pid;GetWindowThreadProcessId(w,out pid);Rect r;GetWindowRect(w,out r);var c=new StringBuilder(256);GetClassName(w,c,256);result.Add(new {hwnd=w.ToInt64(),pid=pid,name=c.ToString(),visible=IsWindowVisible(w),left=r.Left,top=r.Top,width=r.Right-r.Left,height=r.Bottom-r.Top,owned=GetProp(w,"YingliuNativeOwner")!=IntPtr.Zero});return true;},IntPtr.Zero);return result;}
}
'@
$app=Get-Process -Id $ApplicationPid
$handle=if($WindowHandle){[IntPtr]$WindowHandle}else{$app.MainWindowHandle}
$ownerPid=0
[void][NativeVideoCheck]::GetWindowThreadProcessId($handle,[ref]$ownerPid)
if($ownerPid -ne $ApplicationPid){throw 'Window does not belong to this test application'}
$children=[NativeVideoCheck]::Children($handle)
if ($Freeze) {
 $owned = Get-CimInstance Win32_Process -Filter "Name='mpv.exe'" | Where-Object { $_.ParentProcessId -eq $ApplicationPid }
 foreach($p in $owned){$engine=Get-Process -Id $p.ProcessId;[void][NativeVideoCheck]::NtSuspendProcess($engine.Handle)}
}
if($StopEngine){
 Get-CimInstance Win32_Process -Filter "Name='mpv.exe'" | Where-Object { $_.ParentProcessId -eq $ApplicationPid } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
}
if($Screenshot){
 [void][NativeVideoCheck]::SetForegroundWindow($handle)
 Start-Sleep -Milliseconds 200
 $rect=New-Object NativeVideoCheck+Rect
 [void][NativeVideoCheck]::GetWindowRect($handle,[ref]$rect)
 $bitmap=New-Object System.Drawing.Bitmap(($rect.Right-$rect.Left),($rect.Bottom-$rect.Top))
 $graphics=[System.Drawing.Graphics]::FromImage($bitmap)
 try {$graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size);$bitmap.Save($Screenshot)} finally{$graphics.Dispose();$bitmap.Dispose()}
}
@{children=$children;processes=@(Get-CimInstance Win32_Process -Filter "Name='mpv.exe'" | Where-Object {$_.ParentProcessId -eq $ApplicationPid} | Select-Object ProcessId,ParentProcessId,ExecutablePath)} | ConvertTo-Json -Depth 5 -Compress
