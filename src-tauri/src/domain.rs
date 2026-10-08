use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type AppResult<T> = Result<T, String>;

pub fn validate_url(raw: &str) -> AppResult<String> {
    let raw = raw.trim();
    if raw.chars().any(char::is_control) || raw.len() > 8192 {
        return Err("链接包含无效字符或过长".into());
    }
    let parsed = url::Url::parse(raw).map_err(|_| "请输入完整的 http/https 视频链接")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("仅支持不含账号密码的 http/https 视频链接".into());
    }
    Ok(parsed.to_string())
}

pub fn validate_proxy(raw: &str) -> AppResult<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    if raw.chars().any(char::is_control) || raw.len() > 2048 {
        return Err("代理地址包含无效字符".into());
    }
    let parsed = url::Url::parse(raw).map_err(|_| "代理地址格式不正确")?;
    if !matches!(parsed.scheme(), "http" | "https" | "socks5" | "socks5h")
        || parsed.host_str().is_none()
        || !matches!(parsed.path(), "" | "/")
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.port() == Some(0)
    {
        return Err("请输入 HTTP、HTTPS 或 SOCKS5 代理地址，例如 http://127.0.0.1:7890".into());
    }
    Ok(raw.to_string())
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub percent: Option<f64>,
}

pub fn parse_progress(value: &Value) -> Progress {
    let number = |key: &str| value[key].as_f64().filter(|n| n.is_finite() && *n >= 0.0);
    let downloaded = value["downloaded_bytes"].as_u64();
    let total = value["total_bytes"]
        .as_u64()
        .or_else(|| value["total_bytes_estimate"].as_u64())
        .filter(|n| *n > 0);
    Progress {
        downloaded,
        total,
        speed: number("speed"),
        eta: number("eta"),
        percent: downloaded
            .zip(total)
            .map(|(done, total)| (done as f64 / total as f64 * 100.0).clamp(0.0, 100.0)),
    }
}

pub fn redact(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            let cut = ["cookie:", "authorization:", "set-cookie:"]
                .iter()
                .filter_map(|key| lower.find(key))
                .min();
            let line = cut
                .map(|i| format!("{}[登录信息已隐藏]", &line[..i]))
                .unwrap_or_else(|| line.to_string());
            line.split_whitespace()
                .map(|token| {
                    if let Some(scheme) = token.find("://") {
                        let authority = &token[scheme + 3..];
                        let end = authority.find(['/', '?', '#']).unwrap_or(authority.len());
                        let clean = if let Some(at) = authority[..end].rfind('@') {
                            format!("{}[认证已隐藏]{}", &token[..scheme + 3], &authority[at..])
                        } else {
                            token.to_string()
                        };
                        return if let Some(query) = clean.find(['?', '#']) {
                            format!("{}?[参数已隐藏]", &clean[..query])
                        } else {
                            clean
                        };
                    }
                    token.to_string()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_commands_local_files_and_credentials_as_video_urls() {
        for raw in [
            "--exec calc",
            "file:///C:/secret.txt",
            "ftp://example.com/a",
            "http://user:pass@example.com/a",
            "https://example.com/\n--exec calc",
            "https://",
        ] {
            assert!(validate_url(raw).is_err(), "unexpectedly accepted: {raw}");
        }
        assert_eq!(
            validate_url(" https://www.bilibili.com/video/BV123?x=1&y=2 ").unwrap(),
            "https://www.bilibili.com/video/BV123?x=1&y=2"
        );
        assert_eq!(
            validate_url("https://youtu.be/abc").unwrap(),
            "https://youtu.be/abc"
        );
    }

    #[test]
    fn accepts_only_supported_proxy_schemes() {
        assert_eq!(validate_proxy("  ").unwrap(), "");
        assert_eq!(
            validate_proxy("socks5://127.0.0.1:1080").unwrap(),
            "socks5://127.0.0.1:1080"
        );
        assert!(validate_proxy("file:///proxy").is_err());
        assert!(validate_proxy("http://proxy:80/path").is_err());
        assert!(validate_proxy("http://proxy:99999").is_err());
    }

    #[test]
    fn progress_handles_unknown_values_without_fabricating_percent() {
        assert_eq!(
            parse_progress(
                &json!({"downloaded_bytes": 40, "total_bytes": 100, "speed": 12.5, "eta": 4})
            )
            .percent,
            Some(40.0)
        );
        let unknown = parse_progress(
            &json!({"downloaded_bytes": 40, "total_bytes": null, "speed": "NA", "eta": -1}),
        );
        assert_eq!(unknown.downloaded, Some(40));
        assert_eq!(unknown.percent, None);
        assert_eq!(unknown.speed, None);
        assert_eq!(unknown.eta, None);
        assert_eq!(
            parse_progress(&json!({"downloaded_bytes": 200, "total_bytes": 100})).percent,
            Some(100.0)
        );
    }

    #[test]
    fn logs_never_expose_proxy_passwords_or_cookie_values() {
        let clean =
            redact("ERROR proxy http://alice:password@127.0.0.1:7890 Cookie: session=secret");
        assert!(!clean.contains("password"));
        assert!(!clean.contains("session=secret"));
        assert!(clean.contains("ERROR"));
        let signed =
            redact("ERROR fetching https://cdn.example.com/video?token=private&signature=secret");
        assert!(!signed.contains("private"));
        assert!(!signed.contains("secret"));
    }
}
