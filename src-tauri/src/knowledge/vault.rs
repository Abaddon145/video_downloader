//! Vault paths are opened one component at a time relative to directory handles.
//! Path-based canonicalize/check/open and MoveFileEx cannot preserve that boundary.
use crate::domain::AppResult;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    ffi::{c_void, OsString},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    mem::{offset_of, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    sync::Mutex,
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::*,
};

const MAX_MARKDOWN: usize = 5 * 1024 * 1024;
const MAX_TREE: usize = 10_000;
const TRASH: &str = ".yingliu-trash";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub path: String,
    pub text: String,
    pub hash: String,
    pub mtime: i64,
}
#[derive(Debug, Serialize)]
#[serde(tag = "status", content = "document", rename_all = "camelCase")]
pub enum WriteOutcome {
    Saved(Document),
    Conflict(Document),
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub path: String,
    pub name: String,
    pub directory: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreePage {
    pub items: Vec<TreeEntry>,
    pub total: u64,
    pub page: u32,
    pub skipped: usize,
}

pub struct Vault {
    pub id: String,
    pub root: PathBuf,
    // ponytail: serialize one vault's operations; separate vaults remain independent.
    mutation: Mutex<()>,
}

fn component(name: &str) -> AppResult<()> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches([' ', '.'])
        .to_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p)
                .is_some_and(|n| n.chars().count() == 1 && "123456789¹²³".contains(n))
        });
    if name.is_empty()
        || name.starts_with('.')
        || name.ends_with([' ', '.'])
        || name.encode_utf16().count() > 255
        || device
        || name.eq_ignore_ascii_case("node_modules")
        || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
    {
        return Err("路径包含受保护或无效的名称".into());
    }
    Ok(())
}

pub fn validate_relative(rel: &str, allow_root: bool) -> AppResult<String> {
    if rel.is_empty() && allow_root {
        return Ok(String::new());
    }
    if rel.len() > 16_384 {
        return Err("路径过长".into());
    }
    let normalized = rel.replace('\\', "/");
    for name in normalized.split('/') {
        component(name)?;
    }
    Ok(normalized)
}
fn markdown(path: &str) -> AppResult<()> {
    if !path.to_ascii_lowercase().ends_with(".md") {
        return Err("仅支持 Markdown 文件".into());
    }
    Ok(())
}
fn raster(path: &str) -> bool {
    path.rsplit('.').next().is_some_and(|extension| {
        ["png", "jpg", "jpeg", "gif", "webp", "bmp", "avif"]
            .contains(&extension.to_ascii_lowercase().as_str())
    })
}
fn fs_error(error: std::io::Error) -> String {
    format!(
        "知识库文件操作失败：{}",
        error.to_string().chars().take(240).collect::<String>()
    )
}
fn io_error() -> String {
    fs_error(std::io::Error::last_os_error())
}
fn checked(ok: i32) -> AppResult<()> {
    if ok == 0 {
        Err(io_error())
    } else {
        Ok(())
    }
}
fn info(file: &File) -> AppResult<BY_HANDLE_FILE_INFORMATION> {
    let mut result = BY_HANDLE_FILE_INFORMATION::default();
    unsafe {
        checked(GetFileInformationByHandle(
            file.as_raw_handle(),
            &mut result,
        ))?;
    }
    Ok(result)
}
fn final_path(file: &File) -> AppResult<PathBuf> {
    let mut buffer = vec![0u16; 32_768];
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0,
        )
    };
    if length == 0 || length as usize >= buffer.len() {
        return Err("无法确认文件的实际路径".into());
    }
    Ok(PathBuf::from(OsString::from_wide(
        &buffer[..length as usize],
    )))
}
fn inspect(file: &File, internal: Option<&str>) -> AppResult<BY_HANDLE_FILE_INFORMATION> {
    let result = info(file)?;
    if result.dwFileAttributes
        & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
        != 0
    {
        return Err("不能访问链接、隐藏或系统文件".into());
    }
    if result.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0 && result.nNumberOfLinks != 1 {
        return Err("不能访问具有多个硬链接的文件".into());
    }
    // Inspect the long name of this exact handle, including when opened by an 8.3 alias.
    let path = final_path(file)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("文件名不是有效 Unicode")?;
    if let Some(expected) = internal {
        if !name.eq_ignore_ascii_case(expected) {
            return Err("内部路径名称不匹配".into());
        }
    } else {
        component(name)?;
    }
    Ok(result)
}

// NtCreateFile is needed because Win32 CreateFile has no parent-handle parameter.
// These three ABI structs deliberately avoid enabling the whole WDK dependency.
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root_directory: HANDLE,
    object_name: *mut UnicodeString,
    attributes: u32,
    security_descriptor: *mut c_void,
    security_quality_of_service: *mut c_void,
}
#[repr(C)]
struct IoStatusBlock {
    status: usize,
    information: usize,
}
#[link(name = "ntdll")]
extern "system" {
    fn NtCreateFile(
        handle: *mut HANDLE,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatusBlock,
        allocation: *const i64,
        file_attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
        ea: *const c_void,
        ea_length: u32,
    ) -> i32;
    fn RtlNtStatusToDosError(status: i32) -> u32;
    fn NtSetInformationFile(
        handle: HANDLE,
        status: *mut IoStatusBlock,
        information: *const c_void,
        length: u32,
        class: u32,
    ) -> i32;
}
const NT_OPEN: u32 = 1;
const NT_CREATE: u32 = 2;
const NT_OPEN_IF: u32 = 3;

fn child(
    parent: &File,
    name: &str,
    directory: Option<bool>,
    access: u32,
    disposition: u32,
    share: u32,
    internal: Option<&str>,
) -> AppResult<File> {
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    // Callers always pass exactly one validated component, never a relative path.
    if wide.is_empty() || wide.len() > 255 || name.contains(['/', '\\', ':', '\0']) {
        return Err("无效的路径分量".into());
    }
    let mut name = UnicodeString {
        length: (wide.len() * 2) as u16,
        maximum_length: (wide.len() * 2) as u16,
        buffer: wide.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root_directory: parent.as_raw_handle(),
        object_name: &mut name,
        attributes: 0x40 | 0x1000, // OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE
        security_descriptor: std::ptr::null_mut(),
        security_quality_of_service: std::ptr::null_mut(),
    };
    let mut status = IoStatusBlock {
        status: 0,
        information: 0,
    };
    let mut handle = std::ptr::null_mut();
    let options = 0x0020_0000 | 0x20 // FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT
        | match directory { Some(true) => 1, Some(false) => 0x40, None => 0 };
    let result = unsafe {
        NtCreateFile(
            &mut handle,
            access | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            &mut attributes,
            &mut status,
            std::ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            share,
            disposition,
            options,
            std::ptr::null(),
            0,
        )
    };
    if result < 0 {
        return Err(fs_error(std::io::Error::from_raw_os_error(unsafe {
            RtlNtStatusToDosError(result)
        }
            as i32)));
    }
    let file = unsafe { File::from_raw_handle(handle) };
    inspect(&file, internal)?;
    Ok(file)
}

fn open_root(root: &Path) -> AppResult<File> {
    let drive = match root.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) if root.is_absolute() => drive,
            _ => return Err("知识库必须位于本地磁盘，不能使用 UNC 或设备路径".into()),
        },
        _ => return Err("知识库必须位于本地磁盘，不能使用 UNC 或设备路径".into()),
    };
    let drive_path = [drive as u16, ':' as u16, '\\' as u16, 0];
    // GetDriveType values: removable=2, fixed=3, RAM disk=6; reject network drives.
    if !matches!(unsafe { GetDriveTypeW(drive_path.as_ptr()) }, 2 | 3 | 6) {
        return Err("知识库必须位于本地磁盘".into());
    }
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io_error());
    }
    let file = unsafe { File::from_raw_handle(handle) };
    let metadata = info(&file)?;
    if metadata.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || metadata.dwFileAttributes
            & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
            != 0
    {
        return Err("知识库根目录已改变或不可访问".into());
    }
    Ok(file)
}

#[cfg(test)]
mod handle_tests {
    use super::*;
    #[test]
    fn parent_reparse_conversion_does_not_redirect_an_already_open_handle() {
        #[link(name = "kernel32")]
        extern "system" {
            fn DeviceIoControl(
                handle: HANDLE,
                code: u32,
                input: *const c_void,
                input_size: u32,
                output: *mut c_void,
                output_size: u32,
                returned: *mut u32,
                overlapped: *mut c_void,
            ) -> i32;
        }
        let root = std::env::temp_dir().join(format!(
            "vault-reparse-race-{}",
            crate::ffmpeg::runner::new_id()
        ));
        let outside = root.with_extension("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::create_dir(root.join("folder")).unwrap();
        std::fs::write(outside.join("private.md"), "PRIVATE").unwrap();
        let vault = Vault::open(&root).unwrap();
        let parent = vault.parent("folder/private.md").unwrap();
        let directory: Vec<u16> = root
            .join("folder")
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let raw = unsafe {
            CreateFileW(
                directory.as_ptr(),
                FILE_WRITE_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(
            raw,
            INVALID_HANDLE_VALUE,
            "attribute-only attacker handle: {}",
            io_error()
        );
        let attacker = unsafe { File::from_raw_handle(raw) };
        let substitute: Vec<u16> = format!("\\??\\{}", outside.display())
            .encode_utf16()
            .collect();
        let print: Vec<u16> = outside.as_os_str().encode_wide().collect();
        let mut data = Vec::new();
        data.extend_from_slice(&0xa0000003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
        data.extend_from_slice(
            &((8 + (substitute.len() + print.len() + 2) * 2) as u16).to_le_bytes(),
        );
        data.extend_from_slice(&0u16.to_le_bytes());
        for value in [
            0,
            substitute.len() as u16 * 2,
            (substitute.len() as u16 + 1) * 2,
            print.len() as u16 * 2,
        ] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for value in substitute
            .into_iter()
            .chain(Some(0))
            .chain(print)
            .chain(Some(0))
        {
            data.extend_from_slice(&value.to_le_bytes());
        }
        let mut returned = 0;
        assert_ne!(
            unsafe {
                DeviceIoControl(
                    attacker.as_raw_handle(),
                    0x000900a4,
                    data.as_ptr().cast(),
                    data.len() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            },
            0,
            "reparse conversion must actually occur: {}",
            io_error()
        );
        assert!(
            child(
                parent.handle(),
                "private.md",
                Some(false),
                FILE_READ_DATA,
                NT_OPEN,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None
            )
            .is_err(),
            "relative open followed changed parent's junction"
        );
        assert!(vault.read("folder/private.md").is_err());
        assert_eq!(
            std::fs::read_to_string(outside.join("private.md")).unwrap(),
            "PRIVATE"
        );
        vault.create_file("safe.md", "inside").unwrap();
        let source_parent = vault.parent("safe.md").unwrap();
        let source = child(
            source_parent.handle(),
            "safe.md",
            Some(false),
            FILE_READ_DATA | DELETE,
            NT_OPEN,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
        )
        .unwrap();
        let moved = rename(&source, parent.handle(), "escaped.md", false);
        assert!(
            !outside.join("escaped.md").exists(),
            "relative rename followed changed parent's junction"
        );
        if moved.is_ok() {
            rename(&source, source_parent.handle(), "safe.md", false).unwrap();
        }
        drop(source);
        drop(source_parent);
        drop(parent);
        drop(attacker);
        std::fs::remove_dir(root.join("folder")).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }
    #[test]
    fn opened_parent_cannot_be_replaced_and_reparse_after_release_is_rejected() {
        use std::os::windows::process::CommandExt;
        let root =
            std::env::temp_dir().join(format!("vault-parent-{}", crate::ffmpeg::runner::new_id()));
        let outside = root.with_extension("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::create_dir(root.join("folder")).unwrap();
        std::fs::write(outside.join("private.md"), "outside stays private").unwrap();
        let vault = Vault::open(&root).unwrap();
        let opened = vault.parent("folder/private.md").unwrap();
        assert!(std::fs::rename(root.join("folder"), root.join("retired")).is_err());
        drop(opened);
        std::fs::rename(root.join("folder"), root.join("retired")).unwrap();
        let command = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(root.join("folder"))
            .arg(&outside)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(command.status.success());
        assert!(vault.read("folder/private.md").is_err());
        assert!(vault
            .create_file("folder/new.md", "must not escape")
            .is_err());
        assert_eq!(
            std::fs::read_to_string(outside.join("private.md")).unwrap(),
            "outside stays private"
        );
        assert!(!outside.join("new.md").exists());
        std::fs::remove_dir(root.join("folder")).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }
}
fn identity(root: &Path, file: &File) -> AppResult<String> {
    let metadata = info(file)?;
    if metadata.nFileIndexHigh == 0 && metadata.nFileIndexLow == 0 {
        return Err("当前文件系统无法提供稳定的目录标识，请选择本地 NTFS 知识库".into());
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(format!(
            "{}:{}:{}:{}",
            root.to_string_lossy().to_lowercase(),
            metadata.dwVolumeSerialNumber,
            metadata.nFileIndexHigh,
            metadata.nFileIndexLow
        ))
    ))
}

struct Parent {
    chain: Vec<File>,
    name: String,
}
impl Parent {
    fn handle(&self) -> &File {
        self.chain.last().unwrap()
    }
}

fn rename(file: &File, parent: &File, name: &str, replace: bool) -> AppResult<()> {
    let wide: Vec<u16> = name.encode_utf16().collect();
    let bytes = size_of::<FILE_RENAME_INFO>() + wide.len() * 2;
    let mut buffer = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let data = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    unsafe {
        (*data).Anonymous.ReplaceIfExists = replace;
        (*data).RootDirectory = parent.as_raw_handle();
        (*data).FileNameLength = (wide.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            std::ptr::addr_of_mut!((*data).FileName).cast::<u16>(),
            wide.len(),
        );
        // This test host rejects the Win32 wrapper's relative-root form (error 87).
        // NtSetInformationFile preserves the documented HANDLE boundary.
        let mut status = IoStatusBlock {
            status: 0,
            information: 0,
        };
        let result = NtSetInformationFile(
            file.as_raw_handle(),
            &mut status,
            data.cast(),
            bytes as u32,
            10,
        );
        if result < 0 {
            Err(fs_error(std::io::Error::from_raw_os_error(
                RtlNtStatusToDosError(result) as i32,
            )))
        } else {
            Ok(())
        }
    }
}
fn remove_temporary(file: &File) {
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        );
    }
}

fn names(directory: &File) -> AppResult<Vec<String>> {
    let mut result = Vec::new();
    let mut buffer = vec![0u64; 8192];
    let capacity = buffer.len() * size_of::<u64>();
    let mut class = FileIdBothDirectoryRestartInfo;
    loop {
        let ok = unsafe {
            GetFileInformationByHandleEx(
                directory.as_raw_handle(),
                class,
                buffer.as_mut_ptr().cast(),
                capacity as u32,
            )
        };
        if ok == 0 {
            if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                break;
            }
            return Err(io_error());
        }
        class = FileIdBothDirectoryInfo;
        let mut offset = 0;
        loop {
            if offset + size_of::<FILE_ID_BOTH_DIR_INFO>() > capacity {
                return Err("目录返回了无效数据".into());
            }
            let data = unsafe {
                &*buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<FILE_ID_BOTH_DIR_INFO>()
            };
            let length = data.FileNameLength as usize;
            let start = offset + offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
            if length % 2 != 0 || start + length > capacity {
                return Err("目录名称过长".into());
            }
            let wide = unsafe {
                std::slice::from_raw_parts(
                    buffer.as_ptr().cast::<u8>().add(start).cast::<u16>(),
                    length / 2,
                )
            };
            let name = String::from_utf16(wide).map_err(|_| "目录包含无效 Unicode 文件名")?;
            if name != "." && name != ".." {
                result.push(name);
            }
            if result.len() > MAX_TREE {
                return Err("目录超过 10000 项，请选择更小的目录".into());
            }
            if data.NextEntryOffset == 0 {
                break;
            }
            if data.NextEntryOffset as usize % 8 != 0
                || (data.NextEntryOffset as usize) < size_of::<FILE_ID_BOTH_DIR_INFO>()
            {
                return Err("目录返回了无效偏移".into());
            }
            offset += data.NextEntryOffset as usize;
        }
    }
    Ok(result)
}

fn preflight(directory: &File, depth: usize, count: &mut usize) -> AppResult<()> {
    if depth > 64 {
        return Err("目录层级超过 64 层".into());
    }
    // This bounded scan is not an atomic snapshot against unrelated external writers.
    for name in names(directory)? {
        *count += 1;
        if *count > MAX_TREE {
            return Err("目录超过 10000 项".into());
        }
        component(&name)?;
        let entry = child(
            directory,
            &name,
            None,
            FILE_READ_DATA,
            NT_OPEN,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
        )?;
        if info(&entry)?.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            preflight(&entry, depth + 1, count)?;
        } else if markdown(&name).is_err() && !raster(&name) {
            return Err("目录包含不支持移动的文件".into());
        }
    }
    Ok(())
}
fn read_bytes_from(file: &mut File, max: usize) -> AppResult<Vec<u8>> {
    file.seek(SeekFrom::Start(0)).map_err(fs_error)?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(fs_error)?;
    if bytes.len() > max {
        return Err("文件超过大小限制".into());
    }
    Ok(bytes)
}
fn document(file: &mut File, root: &Path) -> AppResult<Document> {
    let bytes = read_bytes_from(file, MAX_MARKDOWN)?;
    let metadata = inspect(file, None)?;
    let actual = final_path(file)?;
    let relative = actual
        .strip_prefix(root)
        .map_err(|_| "文件的实际路径不在当前知识库中")?;
    let path = validate_relative(relative.to_str().ok_or("文件名不是有效 Unicode")?, false)?;
    let ticks = (metadata.ftLastWriteTime.dwHighDateTime as u64) << 32
        | metadata.ftLastWriteTime.dwLowDateTime as u64;
    Ok(Document {
        path,
        hash: format!("{:x}", Sha256::digest(&bytes)),
        text: String::from_utf8(bytes).map_err(|_| "Markdown 必须是 UTF-8 编码")?,
        mtime: (ticks / 10_000).saturating_sub(11_644_473_600_000) as i64,
    })
}

impl Vault {
    pub fn open(path: &Path) -> AppResult<Self> {
        match path.components().next() {
            Some(Component::Prefix(prefix))
                if path.is_absolute()
                    && matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => {}
            _ => return Err("知识库必须使用本地磁盘的绝对路径".into()),
        }
        // User selection may resolve a root junction once; children never follow reparse points.
        let root = std::fs::canonicalize(path).map_err(fs_error)?;
        let file = open_root(&root)?;
        let root = final_path(&file)?;
        if let Some(name) = root.file_name().and_then(|s| s.to_str()) {
            component(name)?;
        }
        let id = identity(&root, &file)?;
        Ok(Self {
            id,
            root,
            mutation: Mutex::new(()),
        })
    }
    fn root_handle(&self) -> AppResult<File> {
        let file = open_root(&self.root)?;
        if identity(&self.root, &file)? != self.id {
            return Err("知识库根目录已被替换，请重新选择".into());
        }
        Ok(file)
    }
    fn parent(&self, path: &str) -> AppResult<Parent> {
        let mut parts: Vec<&str> = path.split('/').collect();
        let name = parts.pop().ok_or("缺少文件名")?.to_string();
        let mut chain = vec![self.root_handle()?];
        for part in parts {
            let entry = child(
                chain.last().unwrap(),
                part,
                Some(true),
                FILE_LIST_DIRECTORY,
                NT_OPEN,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
            )?;
            chain.push(entry);
        }
        Ok(Parent { chain, name })
    }
    fn open_document(&self, path: &str) -> AppResult<(Parent, File)> {
        let parent = self.parent(path)?;
        let file = child(
            parent.handle(),
            &parent.name,
            Some(false),
            FILE_READ_DATA,
            NT_OPEN,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
        )?;
        Ok((parent, file))
    }
    pub fn read(&self, rel: &str) -> AppResult<Document> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let path = validate_relative(rel, false)?;
        markdown(&path)?;
        let (_parent, mut file) = self.open_document(&path)?;
        document(&mut file, &self.root)
    }
    pub fn read_bytes(&self, rel: &str, max: usize) -> AppResult<Vec<u8>> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let path = validate_relative(rel, false)?;
        if !raster(&path) {
            return Err("仅允许读取栅格图片附件".into());
        }
        let (_parent, mut file) = self.open_document(&path)?;
        read_bytes_from(&mut file, max.min(20 * 1024 * 1024))
    }
    fn save(&self, path: &str, text: &str, expected: Option<&str>) -> AppResult<WriteOutcome> {
        markdown(path)?;
        if text.len() > MAX_MARKDOWN {
            return Err("Markdown 超过 5 MiB 限制".into());
        }
        let parent = self.parent(path)?;
        if let Some(expected) = expected {
            let mut existing = child(
                parent.handle(),
                &parent.name,
                Some(false),
                FILE_READ_DATA,
                NT_OPEN,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
            )?;
            let current = document(&mut existing, &self.root)?;
            if current.hash != expected {
                return Ok(WriteOutcome::Conflict(current));
            }
        }
        let temporary = format!(".yingliu-new-{}.tmp", crate::ffmpeg::runner::new_id());
        let mut file = child(
            parent.handle(),
            &temporary,
            Some(false),
            FILE_READ_DATA | FILE_WRITE_DATA | DELETE,
            NT_CREATE,
            FILE_SHARE_READ,
            Some(&temporary),
        )?;
        let mut published = false;
        let result = (|| {
            file.write_all(text.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(fs_error)?;
            if let Some(expected) = expected {
                // Check immediately before publication. Windows offers no hash-CAS rename;
                // an uncooperative writer can still change the destination after this check.
                let mut existing = child(
                    parent.handle(),
                    &parent.name,
                    Some(false),
                    FILE_READ_DATA,
                    NT_OPEN,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    None,
                )?;
                let current = document(&mut existing, &self.root)?;
                if current.hash != expected {
                    return Ok(WriteOutcome::Conflict(current));
                }
            }
            rename(&file, parent.handle(), &parent.name, expected.is_some())?;
            published = true;
            Ok(WriteOutcome::Saved(document(&mut file, &self.root)?))
        })();
        if !published {
            remove_temporary(&file);
        }
        result
    }
    pub fn create_file(&self, rel: &str, text: &str) -> AppResult<Document> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let path = validate_relative(rel, false)?;
        match self.save(&path, text, None)? {
            WriteOutcome::Saved(document) => Ok(document),
            _ => unreachable!(),
        }
    }
    pub fn create_raster(&self, rel: &str, bytes: &[u8]) -> AppResult<()> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let path = validate_relative(rel, false)?;
        if !raster(&path) || bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
            return Err("图片附件类型或大小无效".into());
        }
        let parent = self.parent(&path)?;
        let temporary = format!(".yingliu-new-{}.tmp", crate::ffmpeg::runner::new_id());
        let mut file = child(
            parent.handle(),
            &temporary,
            Some(false),
            FILE_WRITE_DATA | DELETE,
            NT_CREATE,
            FILE_SHARE_READ,
            Some(&temporary),
        )?;
        let result = (|| {
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(fs_error)?;
            rename(&file, parent.handle(), &parent.name, false)
        })();
        if result.is_err() {
            remove_temporary(&file);
        }
        result
    }
    pub fn write(&self, rel: &str, text: &str, expected_hash: &str) -> AppResult<WriteOutcome> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        self.save(&validate_relative(rel, false)?, text, Some(expected_hash))
    }
    pub fn create_dir(&self, rel: &str) -> AppResult<()> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let parent = self.parent(&validate_relative(rel, false)?)?;
        child(
            parent.handle(),
            &parent.name,
            Some(true),
            FILE_LIST_DIRECTORY,
            NT_CREATE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
        )?;
        Ok(())
    }
    pub fn move_entry(&self, from: &str, to: &str) -> AppResult<()> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let from = validate_relative(from, false)?;
        let to = validate_relative(to, false)?;
        let source = self.parent(&from)?;
        let target = self.parent(&to)?;
        let file = child(
            source.handle(),
            &source.name,
            None,
            FILE_READ_DATA | DELETE,
            NT_OPEN,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
        )?;
        if info(&file)?.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            preflight(&file, 0, &mut 0)?;
        } else {
            markdown(&from)?;
            markdown(&to)?;
        }
        rename(&file, target.handle(), &target.name, false)
    }
    pub fn trash(&self, rel: &str, expected_hash: &str) -> AppResult<String> {
        self.trash_kind(rel, expected_hash, false)
    }
    pub fn trash_raster(&self, rel: &str, expected_hash: &str) -> AppResult<String> {
        self.trash_kind(rel, expected_hash, true)
    }
    fn trash_kind(&self, rel: &str, expected_hash: &str, allow_raster: bool) -> AppResult<String> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let path = validate_relative(rel, false)?;
        let parent = self.parent(&path)?;
        let mut file = child(
            parent.handle(),
            &parent.name,
            None,
            FILE_READ_DATA | DELETE,
            NT_OPEN,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
        )?;
        if info(&file)?.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            if !expected_hash.is_empty() {
                return Err("目录回收不接受文件内容哈希".into());
            }
            preflight(&file, 0, &mut 0)?;
        } else {
            if allow_raster {
                if !raster(&path) {
                    return Err("附件不是允许的图片类型".into());
                }
                if format!(
                    "{:x}",
                    Sha256::digest(read_bytes_from(&mut file, 8 * 1024 * 1024)?)
                ) != expected_hash
                {
                    return Err("附件已变化，请重新确认".into());
                }
            } else if {
                markdown(&path)?;
                document(&mut file, &self.root)?.hash != expected_hash
            } {
                return Err("文件已被外部修改，请重新读取后再回收".into());
            }
        }
        let root = self.root_handle()?;
        let trash = child(
            &root,
            TRASH,
            Some(true),
            FILE_LIST_DIRECTORY,
            NT_OPEN_IF,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            Some(TRASH),
        )?;
        let destination = format!("{}-{}", crate::ffmpeg::runner::new_id(), parent.name);
        rename(&file, &trash, &destination, false)?;
        Ok(format!("{TRASH}/{destination}"))
    }
    pub fn list(&self, parent: &str, page: u32) -> AppResult<TreePage> {
        let _lock = self.mutation.lock().map_err(|_| "知识库暂时不可用")?;
        let path = validate_relative(parent, true)?;
        let mut chain = vec![self.root_handle()?];
        if !path.is_empty() {
            for name in path.split('/') {
                chain.push(child(
                    chain.last().unwrap(),
                    name,
                    Some(true),
                    FILE_LIST_DIRECTORY,
                    NT_OPEN,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                )?);
            }
        }
        let directory = chain.last().unwrap();
        let mut items = Vec::new();
        let mut skipped = 0;
        for name in names(directory)? {
            if component(&name).is_err() {
                continue;
            }
            let Ok(file) = child(
                directory,
                &name,
                None,
                FILE_READ_DATA,
                NT_OPEN,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
            ) else {
                skipped += 1;
                continue;
            };
            let directory = info(&file)?.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
            if !directory && markdown(&name).is_err() {
                continue;
            }
            items.push(TreeEntry {
                path: if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}/{name}")
                },
                name,
                directory,
            });
        }
        items.sort_by(|a, b| {
            b.directory
                .cmp(&a.directory)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.name.cmp(&b.name))
        });
        let total = items.len() as u64;
        let page = page.max(1);
        let start = (page as usize - 1).saturating_mul(50);
        Ok(TreePage {
            items: items.into_iter().skip(start).take(50).collect(),
            total,
            page,
            skipped,
        })
    }
}
