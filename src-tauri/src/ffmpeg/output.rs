use super::{command::valid_path, models::*};
use crate::domain::AppResult;
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};
pub fn input(text: &str) -> AppResult<PathBuf> {
    valid_path(text)?;
    let path = fs::canonicalize(text).map_err(|_| "找不到输入文件")?;
    if !path.is_file() {
        return Err("请选择普通媒体文件".into());
    }
    valid_path(&path.to_string_lossy())?;
    Ok(path)
}
pub fn directory(text: &str) -> AppResult<PathBuf> {
    valid_path(text)?;
    let path = fs::canonicalize(text).map_err(|_| "找不到输出目录")?;
    if !path.is_dir() {
        return Err("请选择有效的输出目录".into());
    }
    valid_path(&path.to_string_lossy())?;
    let trial = path.join(format!(".media-write-check-{}", super::runner::new_id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&trial)
        .map_err(|_| "当前目录不可写，请更换保存位置")?;
    drop(file);
    fs::remove_file(trial).map_err(|_| "当前目录不可写，请更换保存位置")?;
    Ok(path)
}
pub fn safe_stem(text: &str) -> String {
    let mut name: String = text
        .chars()
        .take(60)
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    name = name.trim_matches([' ', '.']).to_string();
    if name.is_empty() {
        name = "media".into();
    }
    let head = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&head.as_str())
    {
        name.insert(0, '_');
    }
    name
}
pub fn stem(r: &MediaRequest) -> String {
    let base = safe_stem(
        &Path::new(&r.input_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy(),
    );
    let suffix = match r.operation {
        MediaOperation::Remux => "remuxed".into(),
        MediaOperation::Transcode => "converted".into(),
        MediaOperation::Trim => "trimmed".into(),
        MediaOperation::ExtractAudio => "audio".into(),
        MediaOperation::Screenshot => {
            let ms = (r.start * 1000.).round() as u64;
            format!(
                "{:02}-{:02}-{:02}-{:03}",
                ms / 3600000,
                ms / 60000 % 60,
                ms / 1000 % 60,
                ms % 1000
            )
        }
    };
    format!("{base}-{suffix}")
}
pub fn temporary(directory: &Path, id: &str) -> AppResult<PathBuf> {
    if id.len() > 80 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("媒体任务标识无效".into());
    }
    let root = directory.join(".video-downloader/media-temp").join(id);
    fs::create_dir_all(&root).map_err(|_| "当前目录不可写，请更换保存位置")?;
    let actual = fs::canonicalize(&root).map_err(|_| "媒体临时目录无法访问")?;
    if !actual.starts_with(directory) {
        return Err("媒体临时目录不在保存目录中".into());
    }
    Ok(actual)
}
pub fn publish(
    temp: &Path,
    directory: &Path,
    name: &str,
    format: OutputFormat,
) -> AppResult<PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
    if !temp.is_file() || fs::metadata(temp).map_err(|_| "无法读取处理结果")?.len() == 0 {
        return Err("处理结果为空，原始文件已保留".into());
    }
    let name = safe_stem(name);
    for n in 0..10000 {
        let filename = if n == 0 {
            format!("{name}.{}", format.extension())
        } else {
            format!("{name} ({n}).{}", format.extension())
        };
        let destination = directory.join(filename);
        if destination == temp {
            return Err("发布路径不能是临时文件".into());
        }
        let wide = |p: &Path| {
            p.as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let ok = unsafe {
            MoveFileExW(
                wide(temp).as_ptr(),
                wide(&destination).as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok != 0 {
            return Ok(destination);
        }
        let e = std::io::Error::last_os_error();
        if e.kind() == std::io::ErrorKind::AlreadyExists
            || matches!(e.raw_os_error(), Some(80 | 183))
        {
            continue;
        }
        return Err(match e.raw_os_error() {
            Some(112) => "磁盘剩余空间不足",
            Some(5) => "当前目录不可写，请更换保存位置",
            _ => "无法保存处理结果，请检查文件权限和磁盘空间",
        }
        .into());
    }
    Err("同名文件过多，请更换保存目录".into())
}
pub fn cleanup(directory: &Path, id: &str) {
    let root = directory.join(".video-downloader/media-temp").join(id);
    if let Ok(actual) = fs::canonicalize(&root) {
        if actual.starts_with(directory) && actual == root {
            let _ = fs::remove_dir_all(actual);
        }
    }
}
