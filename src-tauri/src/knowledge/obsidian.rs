use super::models::{ObsidianBinding, ObsidianConnectionState, ObsidianKind};
use crate::domain::AppResult;
use std::path::{Component, Path, Prefix};

pub fn local_path(absolute: &Path) -> AppResult<&str> {
    match absolute.components().next() {
        Some(Component::Prefix(prefix))
            if absolute.is_absolute()
                && matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => {}
        _ => return Err("Obsidian 仅支持本地知识库的绝对路径".into()),
    }
    let path = absolute
        .to_str()
        .filter(|s| !s.contains('\0'))
        .ok_or("Obsidian 路径无效")?;
    if let Some(drive) = path.strip_prefix(r"\\?\") {
        let bytes = drive.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'\\'
        {
            return Err("Obsidian 仅支持本地知识库路径".into());
        }
        Ok(drive)
    } else {
        Ok(path)
    }
}

pub fn path_uri(absolute: &Path) -> AppResult<String> {
    let path = local_path(absolute)?;
    let mut uri = url::Url::parse("obsidian://open").map_err(|_| "无法创建 Obsidian 链接")?;
    uri.query_pairs_mut().append_pair("path", path);
    Ok(uri.into())
}

pub fn vault_uri(identifier: &str, relative: &str) -> AppResult<String> {
    if identifier.trim().is_empty()
        || identifier.len() > 256
        || identifier.chars().any(char::is_control)
    {
        return Err("Obsidian Vault 标识无效".into());
    }
    let relative = super::vault::validate_relative(relative, false)?;
    if !relative.to_ascii_lowercase().ends_with(".md") {
        return Err("仅支持 Markdown 笔记".into());
    }
    let mut uri = url::Url::parse("obsidian://open").map_err(|_| "无法创建 Obsidian 链接")?;
    uri.query_pairs_mut()
        .append_pair("vault", identifier)
        .append_pair("file", &relative);
    Ok(uri.into())
}

pub fn validate_binding(binding: &ObsidianBinding) -> AppResult<()> {
    match binding.kind {
        ObsidianKind::AbsolutePath => {
            if binding.obsidian_vault_identifier.is_some() {
                return Err("绝对路径兼容方式不接受 Vault 标识".into());
            }
        }
        ObsidianKind::VaultId => {
            let id = binding
                .obsidian_vault_identifier
                .as_deref()
                .unwrap_or_default();
            if id.len() != 16 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err("请从 Obsidian Vault 切换器复制 16 位 Vault ID".into());
            }
        }
        ObsidianKind::VaultName => {
            let name = binding
                .obsidian_vault_identifier
                .as_deref()
                .unwrap_or_default();
            if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
                return Err("请明确填写并确认 Obsidian Vault 名称".into());
            }
        }
    }
    Ok(())
}

pub fn connection_state(
    binding: Option<&ObsidianBinding>,
    root_id: &str,
    protocol: bool,
) -> ObsidianConnectionState {
    if binding.is_some_and(|b| b.yingliu_vault_id != root_id) {
        return ObsidianConnectionState::StaleVault;
    }
    if !protocol {
        return ObsidianConnectionState::ProtocolUnavailable;
    }
    match binding {
        None => ObsidianConnectionState::Unconfigured,
        Some(b) if !b.registration_confirmed || validate_binding(b).is_err() => {
            ObsidianConnectionState::RegistrationPending
        }
        Some(b) if b.confirmed_by_user => ObsidianConnectionState::UserConfirmed,
        Some(_) => ObsidianConnectionState::VerificationPending,
    }
}

pub fn protocol_available() -> bool {
    use windows_sys::Win32::UI::Shell::{
        AssocQueryStringW, ASSOCF_IS_PROTOCOL, ASSOCSTR_EXECUTABLE,
    };
    let scheme: Vec<u16> = "obsidian\0".encode_utf16().collect();
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    // Query the registered protocol handler only; never launch or register an application here.
    unsafe {
        AssocQueryStringW(
            ASSOCF_IS_PROTOCOL,
            ASSOCSTR_EXECUTABLE,
            scheme.as_ptr(),
            std::ptr::null(),
            buffer.as_mut_ptr(),
            &mut length,
        ) == 0
            && length > 1
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uri_round_trips_reserved_characters() {
        let p = Path::new("C:/Vault 中文/a #&?.md");
        let u = url::Url::parse(&path_uri(p).unwrap()).unwrap();
        assert_eq!(
            u.query_pairs().find(|(k, _)| k == "path").unwrap().1,
            p.to_string_lossy()
        );
        assert!(path_uri(Path::new("relative.md")).is_err());
    }

    #[test]
    fn compatibility_uri_rejects_network_paths() {
        assert!(path_uri(Path::new(r"\\server\share\note.md")).is_err());
    }

    #[test]
    fn vault_uri_round_trips_identifier_and_relative_path_without_double_encoding() {
        let file = "映流/课程 空格/#&%25😀.md";
        let url = url::Url::parse(&vault_uri("ef6ca3e3b524d22f", file).unwrap()).unwrap();
        let pairs = url
            .query_pairs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(pairs["vault"], "ef6ca3e3b524d22f");
        assert_eq!(pairs["file"], file);
        assert!(!pairs.contains_key("path"));
        for bad in [
            "../outside.md",
            "C:/outside.md",
            "folder/../outside.md",
            "note.txt",
            ".obsidian/settings.md",
        ] {
            assert!(vault_uri("ef6ca3e3b524d22f", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn states_distinguish_registration_verification_and_stale_root() {
        use ObsidianConnectionState::*;
        let mut b = ObsidianBinding {
            yingliu_vault_id: "local".into(),
            kind: ObsidianKind::VaultId,
            obsidian_vault_identifier: Some("ef6ca3e3b524d22f".into()),
            registration_confirmed: false,
            confirmed_by_user: false,
        };
        assert_eq!(connection_state(None, "local", true), Unconfigured);
        assert_eq!(
            connection_state(Some(&b), "local", false),
            ProtocolUnavailable
        );
        assert_eq!(
            connection_state(Some(&b), "local", true),
            RegistrationPending
        );
        b.registration_confirmed = true;
        assert_eq!(
            connection_state(Some(&b), "local", true),
            VerificationPending
        );
        b.confirmed_by_user = true;
        assert_eq!(connection_state(Some(&b), "local", true), UserConfirmed);
        assert_eq!(connection_state(Some(&b), "other", true), StaleVault);
        assert_eq!(connection_state(Some(&b), "other", false), StaleVault);
    }
}

#[cfg(test)]
mod canonical_tests {
    #[test]
    fn windows_verbatim_drive_path_becomes_normal_obsidian_path() {
        let value = super::path_uri(std::path::Path::new(r"\\?\C:\Vault 中文\note.md")).unwrap();
        let u = url::Url::parse(&value).unwrap();
        assert_eq!(
            u.query_pairs().find(|(k, _)| k == "path").unwrap().1,
            r"C:\Vault 中文\note.md"
        );
        assert!(super::path_uri(std::path::Path::new(r"\\?\UNC\server\share\note.md")).is_err());
    }
}
