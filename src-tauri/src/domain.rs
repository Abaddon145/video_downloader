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

pub fn cookie_summary(text: &str) -> AppResult<crate::models::CookieSummary> {
    if text.len() >= 2 * 1024 * 1024
        || !text.lines().any(|line| {
            line.trim_start_matches('\u{feff}').starts_with('#')
                && line.contains("HTTP Cookie File")
        })
    {
        return Err("请选择小于 2 MiB 的 Netscape 格式 Cookie 文件".into());
    }
    let mut summary = crate::models::CookieSummary::default();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_start_matches('\u{feff}');
        if line.is_empty() || (line.starts_with('#') && !line.starts_with("#HttpOnly_")) {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        let invalid = || {
            format!(
                "Cookie 文件第 {} 行格式无效，请重新导出（需要 7 列和有效的过期时间）",
                index + 1
            )
        };
        if fields.len() != 7
            || fields[0].trim_start_matches("#HttpOnly_").is_empty()
            || !matches!(fields[1], "TRUE" | "FALSE")
            || !fields[2].starts_with('/')
            || !matches!(fields[3], "TRUE" | "FALSE")
        {
            return Err(invalid());
        }
        let expiry: u64 = fields[4].parse().map_err(|_| invalid())?;
        if expiry > 253402300799 {
            return Err(invalid());
        }
        summary.count += 1;
        if expiry == 0 {
            summary.session_count += 1;
        } else {
            summary.earliest_expiry = Some(
                summary
                    .earliest_expiry
                    .map_or(expiry, |previous| previous.min(expiry)),
            );
            summary.latest_expiry = Some(
                summary
                    .latest_expiry
                    .map_or(expiry, |previous| previous.max(expiry)),
            );
        }
    }
    if summary.count == 0 {
        return Err("Cookie 文件没有可用记录，请重新导出".into());
    }
    Ok(summary)
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
    fn cookie_expiry_parses_normal_comments_http_only_and_sessions_without_values() {
        let text = "# Netscape HTTP Cookie File\n# comment\n.example.com\tTRUE\t/\tFALSE\t1900000000\tfixture\tsynthetic-tripwire\n#HttpOnly_.example.com\tTRUE\t/\tTRUE\t1800000000\tfixture\tsynthetic-tripwire\n.example.com\tTRUE\t/\tFALSE\t0\tfixture\tsynthetic-tripwire\n";
        let summary = cookie_summary(text).unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.session_count, 1);
        assert_eq!(summary.earliest_expiry, Some(1800000000));
        assert_eq!(summary.latest_expiry, Some(1900000000));
        let serialized = serde_json::to_string(&summary).unwrap();
        assert!(
            !serialized.contains("synthetic-tripwire")
                && !serialized.contains("fixture")
                && !serialized.contains("example.com")
        );
        let session = cookie_summary(
            "# HTTP Cookie File\n.example.com\tFALSE\t/\tFALSE\t0\tfixture\ttripwire",
        )
        .unwrap();
        assert_eq!(session.latest_expiry, None);
    }
    #[test]
    fn cookie_expiry_rejects_empty_bad_columns_and_timestamps_without_leaking_rows() {
        for text in ["", "# Netscape HTTP Cookie File\n# no rows", "# Netscape HTTP Cookie File\nsynthetic-tripwire", "# Netscape HTTP Cookie File\n.example.com\tTRUE\t/\tFALSE\tbad\tfixture\tsynthetic-tripwire", "# Netscape HTTP Cookie File\n.example.com\tTRUE\t/\tFALSE\t-1\tfixture\tsynthetic-tripwire"] {
            let error = cookie_summary(text).unwrap_err(); assert!(!error.contains("synthetic-tripwire"));
        }
    }
    #[test]
    fn legacy_settings_have_no_cookie_statistics_until_reimport() {
        let value = json!({"downloadDir":"C:/saved","concurrency":2,"cookieMode":"file","browser":"edge","browserProfile":"","hasCookieFile":true,"proxyEnabled":false,"proxyUrl":""});
        let settings: crate::models::AppSettings = serde_json::from_value(value).unwrap();
        assert!(settings.has_cookie_file);
        assert!(settings.cookie_summary.is_none());
    }

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
