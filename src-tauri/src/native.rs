use crate::domain::AppResult;
use std::os::windows::{
    ffi::OsStrExt,
    io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
use windows_sys::Win32::{
    Foundation::{LocalFree, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0},
    Security::{
        Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
        SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
    System::{JobObjects::*, Pipes::CreatePipe, Threading::*},
};

fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
fn last_error() -> String {
    std::io::Error::last_os_error().to_string()
}
fn checked(ok: i32) -> AppResult<()> {
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

pub fn message(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(std::ffi::OsStr::new(text)).as_ptr(),
            wide(std::ffi::OsStr::new("视频下载器")).as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub fn single_instance() -> AppResult<Option<OwnedHandle>> {
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
        UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE},
    };
    let name = format!(
        "Local\\VideoDownloader-{}",
        std::env::var("VIDEO_DOWNLOADER_DATA_DIR")
            .unwrap_or_default()
            .replace(['\\', '/'], "_")
    );
    unsafe {
        let mutex = CreateMutexW(
            std::ptr::null(),
            0,
            wide(std::ffi::OsStr::new(&name)).as_ptr(),
        );
        if mutex.is_null() {
            return Err(last_error());
        }
        let already_exists = GetLastError() == ERROR_ALREADY_EXISTS;
        let mutex = OwnedHandle::from_raw_handle(mutex);
        if already_exists {
            let window = FindWindowW(
                std::ptr::null(),
                wide(std::ffi::OsStr::new("视频下载器")).as_ptr(),
            );
            if !window.is_null() {
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
            Ok(None)
        } else {
            Ok(Some(mutex))
        }
    }
}

pub fn quote_argument(value: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        result.push_str(&"\\".repeat(if character == '"' {
            slashes * 2 + 1
        } else {
            slashes
        }));
        result.push(character);
        slashes = 0;
    }
    result.push_str(&"\\".repeat(slashes * 2));
    result.push('"');
    result
}

fn crypt(bytes: &[u8], decrypt: bool) -> AppResult<Vec<u8>> {
    if bytes.len() > u32::MAX as usize {
        return Err("敏感数据过大".into());
    }
    let source = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut destination: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    unsafe {
        checked(if decrypt {
            CryptUnprotectData(
                &source,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut destination,
            )
        } else {
            CryptProtectData(
                &source,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut destination,
            )
        })?;
        let result =
            std::slice::from_raw_parts(destination.pbData, destination.cbData as usize).to_vec();
        LocalFree(destination.pbData.cast());
        Ok(result)
    }
}
pub fn protect(bytes: &[u8]) -> AppResult<Vec<u8>> {
    crypt(bytes, false)
}
pub fn unprotect(bytes: &[u8]) -> AppResult<Vec<u8>> {
    crypt(bytes, true)
}

pub fn replace_file(from: &Path, to: &Path) -> AppResult<()> {
    unsafe {
        checked(MoveFileExW(
            wide(from.as_os_str()).as_ptr(),
            wide(to.as_os_str()).as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ))
    }
}
pub fn activate_checked(
    candidate: &Path,
    current: &Path,
    check: impl Fn(&Path) -> AppResult<()>,
) -> AppResult<()> {
    check(candidate)?;
    let backup = current.with_extension("backup.exe");
    fs::copy(current, &backup).map_err(|e| e.to_string())?;
    replace_file(candidate, current)?;
    if let Err(error) = check(current) {
        let restored = current.with_extension("restore.exe");
        fs::copy(&backup, &restored)
            .map_err(|e| format!("回滚文件无法准备：{e}；启动错误：{error}"))?;
        replace_file(&restored, current)?;
        return Err(format!("新内核启动检查失败，已恢复旧版本：{error}"));
    }
    Ok(())
}
fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let temporary = path.with_extension("new");
    let mut file = File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    replace_file(&temporary, path)
}
pub fn save_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Ok(previous) = fs::read(path) {
        if serde_json::from_slice::<serde_json::Value>(&previous).is_ok() {
            write_atomic(&path.with_extension("bak"), &previous)?;
        }
    }
    write_atomic(path, bytes)
}
pub fn load_with_backup(path: &Path) -> AppResult<serde_json::Value> {
    for candidate in [path.to_path_buf(), path.with_extension("bak")] {
        if let Ok(bytes) = fs::read(candidate) {
            if let Ok(value) = serde_json::from_slice(&bytes) {
                return Ok(value);
            }
        }
    }
    Err("设置文件及备份均无法读取，请保留数据文件后重新启动".into())
}

pub struct Job {
    handle: OwnedHandle,
}
impl Job {
    fn new() -> AppResult<Arc<Self>> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(last_error());
            }
            let job = Arc::new(Self {
                handle: OwnedHandle::from_raw_handle(handle),
            });
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            checked(SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            ))?;
            Ok(job)
        }
    }
    pub fn terminate(&self) -> AppResult<()> {
        unsafe { checked(TerminateJobObject(self.handle.as_raw_handle(), 1)) }
    }
}

pub struct RunningProcess {
    pub job: Arc<Job>,
    handle: OwnedHandle,
}
impl RunningProcess {
    pub fn wait(&self) -> AppResult<u32> {
        unsafe {
            if WaitForSingleObject(self.handle.as_raw_handle(), INFINITE) != WAIT_OBJECT_0 {
                return Err(last_error());
            }
            let mut code = 1;
            checked(GetExitCodeProcess(self.handle.as_raw_handle(), &mut code))?;
            Ok(code)
        }
    }
}
fn pipe() -> AppResult<(File, OwnedHandle)> {
    unsafe {
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: 1,
        };
        let mut read: HANDLE = std::ptr::null_mut();
        let mut write: HANDLE = std::ptr::null_mut();
        checked(CreatePipe(&mut read, &mut write, &attributes, 0))?;
        let read = File::from_raw_handle(read);
        let write = OwnedHandle::from_raw_handle(write);
        checked(SetHandleInformation(
            read.as_raw_handle(),
            HANDLE_FLAG_INHERIT,
            0,
        ))?;
        Ok((read, write))
    }
}
pub fn spawn(exe: &Path, args: &[String]) -> AppResult<(RunningProcess, File, File)> {
    spawn_in(exe, args, None)
}
pub fn spawn_in(
    exe: &Path,
    args: &[String],
    directory: Option<&Path>,
) -> AppResult<(RunningProcess, File, File)> {
    let current = directory.map(|p| wide(p.as_os_str()));
    if args.iter().any(|s| s.contains('\0')) {
        return Err("参数包含无效字符".into());
    }
    let job = Job::new()?;
    let (stdout, stdout_write) = pipe()?;
    let (stderr, stderr_write) = pipe()?;
    let stdin = File::open("NUL").map_err(|e| e.to_string())?;
    let command = std::iter::once(exe.to_string_lossy().into_owned())
        .chain(args.iter().cloned())
        .map(|s| quote_argument(&s))
        .collect::<Vec<_>>()
        .join(" ");
    let mut command = wide(std::ffi::OsStr::new(&command));
    unsafe {
        checked(SetHandleInformation(
            stdin.as_raw_handle(),
            HANDLE_FLAG_INHERIT,
            HANDLE_FLAG_INHERIT,
        ))?;
        let mut size = 0;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size);
        let mut buffer = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
        let attributes = buffer.as_mut_ptr().cast();
        checked(InitializeProcThreadAttributeList(
            attributes, 1, 0, &mut size,
        ))?;
        let handles = [
            stdin.as_raw_handle(),
            stdout_write.as_raw_handle(),
            stderr_write.as_raw_handle(),
        ];
        let result = (|| {
            checked(UpdateProcThreadAttribute(
                attributes,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast(),
                std::mem::size_of_val(&handles),
                std::ptr::null_mut(),
                std::ptr::null(),
            ))?;
            let mut startup: STARTUPINFOEXW = std::mem::zeroed();
            startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
            startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
            startup.StartupInfo.hStdInput = handles[0];
            startup.StartupInfo.hStdOutput = handles[1];
            startup.StartupInfo.hStdError = handles[2];
            startup.lpAttributeList = attributes;
            let mut info: PROCESS_INFORMATION = std::mem::zeroed();
            checked(CreateProcessW(
                wide(exe.as_os_str()).as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
                CREATE_NO_WINDOW | CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
                std::ptr::null(),
                current.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
                &startup.StartupInfo,
                &mut info,
            ))?;
            let process = OwnedHandle::from_raw_handle(info.hProcess);
            let thread = OwnedHandle::from_raw_handle(info.hThread);
            if AssignProcessToJobObject(job.handle.as_raw_handle(), process.as_raw_handle()) == 0 {
                let error = last_error();
                TerminateProcess(process.as_raw_handle(), 1);
                return Err(error);
            }
            if ResumeThread(thread.as_raw_handle()) == u32::MAX {
                job.terminate()?;
                return Err(last_error());
            }
            Ok(RunningProcess {
                job,
                handle: process,
            })
        })();
        DeleteProcThreadAttributeList(attributes);
        Ok((result?, stdout, stderr))
    }
}
pub fn read_bounded(mut file: File, limit: usize) -> AppResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count.min(limit.saturating_sub(output.len()))]);
    }
    Ok(output)
}
pub fn capture(exe: &Path, args: &[String]) -> AppResult<String> {
    capture_with(exe, args, |_| {})
}
pub fn capture_with(
    exe: &Path,
    args: &[String],
    on_start: impl FnOnce(Arc<Job>),
) -> AppResult<String> {
    let (process, stdout, stderr) = spawn(exe, args)?;
    on_start(process.job.clone());
    let errors = std::thread::spawn(move || read_bounded(stderr, 64 * 1024));
    // ponytail: previews are bounded to 32 MiB; stream/paginate remote metadata if huge playlists become a real use case.
    let output = read_bounded(stdout, 32 * 1024 * 1024)?;
    let code = process.wait()?;
    let errors = errors.join().map_err(|_| "读取内核日志失败")??;
    if code != 0 {
        return Err(crate::domain::redact(&String::from_utf8_lossy(&errors))
            .trim()
            .to_string());
    }
    Ok(String::from_utf8_lossy(&output)
        .trim_start_matches('\u{feff}')
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_quotes_preserve_spaces_quotes_and_trailing_backslashes() {
        assert_eq!(quote_argument(""), "\"\"");
        assert_eq!(quote_argument("C:\\视频 下载\\"), "\"C:\\视频 下载\\\\\"");
        assert_eq!(quote_argument("say \"hello\""), "\"say \\\"hello\\\"\"");
    }

    #[test]
    fn dpapi_roundtrips_without_storing_plaintext() {
        let clear = b"session=secret; proxy-password=private";
        let sealed = protect(clear).unwrap();
        assert_ne!(sealed, clear);
        assert_eq!(unprotect(&sealed).unwrap(), clear);
        assert!(unprotect(b"invalid sealed data").is_err());
    }

    #[test]
    fn corrupted_state_recovers_last_good_snapshot() {
        let dir = std::env::temp_dir().join(format!(
            "video-downloader-state-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        save_atomic(&path, br#"{"value":1}"#).unwrap();
        save_atomic(&path, br#"{"value":2}"#).unwrap();
        std::fs::write(&path, b"corrupted").unwrap();
        assert_eq!(load_with_backup(&path).unwrap()["value"], 1);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(path.with_extension("bak")).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn captures_utf8_stdout_without_mixing_stderr() {
        let exe = Path::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = capture(exe, &["-NoProfile".into(), "-Command".into(), "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Write-Output '视频 输出'; [Console]::Error.WriteLine('warning')".into()]).unwrap();
        assert!(output.contains("视频 输出"));
        assert!(!output.contains("warning"));
    }

    #[test]
    fn cancelling_job_terminates_the_child_process_tree() {
        use std::io::{BufRead, BufReader};
        let exe = Path::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
        let script = "$child = Start-Process powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 60' -WindowStyle Hidden -PassThru; [Console]::WriteLine($child.Id); Start-Sleep -Seconds 60";
        let (process, output, _errors) = spawn(
            exe,
            &["-NoProfile".into(), "-Command".into(), script.into()],
        )
        .unwrap();
        let mut line = String::new();
        BufReader::new(output).read_line(&mut line).unwrap();
        let pid: u32 = line.trim().parse().unwrap();
        unsafe {
            let child = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            assert!(!child.is_null());
            let child = OwnedHandle::from_raw_handle(child);
            process.job.terminate().unwrap();
            assert_eq!(
                WaitForSingleObject(child.as_raw_handle(), 5000),
                WAIT_OBJECT_0
            );
        }
        assert_ne!(process.wait().unwrap(), 0);
    }

    #[test]
    fn engine_activation_failure_restores_previous_binary() {
        let dir = std::env::temp_dir().join(format!(
            "video-downloader-update-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let current = dir.join("yt-dlp.exe");
        let candidate = dir.join("candidate.exe");
        fs::write(&current, b"known-good").unwrap();
        fs::write(&candidate, b"new-version").unwrap();
        let result = activate_checked(&candidate, &current, |path| {
            if path == &current {
                Err("health check failed".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&current).unwrap(), b"known-good");
        fs::write(&candidate, b"checked-version").unwrap();
        activate_checked(&candidate, &current, |_| Ok(())).unwrap();
        assert_eq!(fs::read(&current).unwrap(), b"checked-version");
        assert_eq!(
            fs::read(current.with_extension("backup.exe")).unwrap(),
            b"known-good"
        );
        fs::remove_file(current.with_extension("backup.exe")).unwrap();
        fs::remove_file(current).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
