use crate::domain::AppResult;
use std::path::Path;
pub fn uri(absolute: &Path) -> AppResult<String> {
    if !absolute.is_absolute() {
        return Err("Obsidian 路径必须是绝对路径".into());
    }
    let path = absolute
        .to_str()
        .filter(|s| !s.contains('\0'))
        .ok_or("Obsidian 路径无效")?;
    let path = if let Some(drive) = path.strip_prefix(r"\\?\") {
        let bytes = drive.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'\\'
        {
            return Err("Obsidian 仅支持本地知识库路径".into());
        }
        drive
    } else {
        path
    };
    let mut uri = url::Url::parse("obsidian://open").map_err(|_| "无法创建 Obsidian 链接")?;
    uri.query_pairs_mut().append_pair("path", path);
    Ok(uri.into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uri_round_trips_reserved_characters() {
        let p = Path::new("C:/Vault 中文/a #&?.md");
        let u = url::Url::parse(&uri(p).unwrap()).unwrap();
        assert_eq!(
            u.query_pairs().find(|(k, _)| k == "path").unwrap().1,
            p.to_string_lossy()
        );
        assert!(uri(Path::new("relative.md")).is_err());
    }
}

#[cfg(test)]
mod canonical_tests {
    #[test]
    fn windows_verbatim_drive_path_becomes_normal_obsidian_path() {
        let value = super::uri(std::path::Path::new(r"\\?\C:\Vault 中文\note.md")).unwrap();
        let u = url::Url::parse(&value).unwrap();
        assert_eq!(
            u.query_pairs().find(|(k, _)| k == "path").unwrap().1,
            r"C:\Vault 中文\note.md"
        );
        assert!(super::uri(std::path::Path::new(r"\\?\UNC\server\share\note.md")).is_err());
    }
}
