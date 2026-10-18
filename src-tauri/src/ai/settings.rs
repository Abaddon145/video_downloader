use super::models::AnalysisMode;
use crate::{domain::AppResult, native};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
pub const DEFAULT_MODEL: &str = "deepseek-flash";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AiSettings {
    pub model: String,
    pub default_mode: AnalysisMode,
    pub whisper_model: String,
    pub allow_transcript: bool,
    pub allow_images: bool,
    pub clean_temporary_files: bool,
    pub auto_generate_notes: bool,
    pub auto_analyze_downloads: bool,
    pub chunk_concurrency: u8,
}
impl Default for AiSettings {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.into(),
            default_mode: AnalysisMode::Standard,
            whisper_model: "base".into(),
            allow_transcript: true,
            allow_images: true,
            clean_temporary_files: true,
            auto_generate_notes: true,
            auto_analyze_downloads: false,
            chunk_concurrency: 1,
        }
    }
}
impl AiSettings {
    pub fn validate(&self) -> AppResult<()> {
        if ![DEFAULT_MODEL, "deepseek-v4-pro"].contains(&self.model.as_str()) {
            return Err("请选择受支持的 DeepSeek 模型".into());
        }
        if self.allow_images && self.model != DEFAULT_MODEL {
            return Err("当前模型未启用图片输入，请使用 DeepSeek Flash 或关闭画面上传".into());
        }
        if !["base", "small", "custom"].contains(&self.whisper_model.as_str()) {
            return Err("语音模型选项无效".into());
        }
        if !(1..=2).contains(&self.chunk_concurrency) {
            return Err("AI请求并发只能为1或2".into());
        }
        Ok(())
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsView {
    pub settings: AiSettings,
    pub has_api_key: bool,
}
pub fn load_settings(data: &Path) -> AppResult<AiSettings> {
    let path = data.join("ai/settings.json");
    if !path.exists() {
        return Ok(AiSettings::default());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取AI设置")?;
    if bytes.len() > 16384 {
        return Err("AI设置文件过大".into());
    }
    let s: AiSettings =
        serde_json::from_slice(&bytes).map_err(|_| "AI设置格式错误，请保留原文件")?;
    s.validate()?;
    Ok(s)
}
pub fn save_settings(data: &Path, s: &AiSettings) -> AppResult<()> {
    s.validate()?;
    fs::create_dir_all(data.join("ai")).map_err(|_| "无法创建AI设置目录")?;
    native::save_atomic(
        &data.join("ai/settings.json"),
        &serde_json::to_vec_pretty(s).map_err(|_| "无法保存AI设置")?,
    )
}
// No Debug/Serialize implementation: secrets cannot enter snapshots or IPC.
pub struct Secret(Vec<u8>);
impl Secret {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[已隐藏密钥]")
    }
}
pub(crate) fn wipe(bytes: &mut [u8]) {
    for b in bytes {
        unsafe {
            std::ptr::write_volatile(b, 0);
        }
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}
fn validate_key(key: &[u8]) -> AppResult<()> {
    if !(16..=512).contains(&key.len()) || !key.iter().all(|b| b.is_ascii_graphic()) {
        return Err("API密钥格式无效，请检查后重新输入".into());
    }
    Ok(())
}
pub(crate) fn store_key(data: &Path, key: &[u8]) -> AppResult<()> {
    validate_key(key)?;
    let encrypted = native::protect(key).map_err(|_| "Windows无法加密API密钥")?;
    let dir = data.join("ai");
    fs::create_dir_all(&dir).map_err(|_| "无法创建密钥目录")?;
    let temp = dir.join(format!("{}.key.tmp", crate::ffmpeg::runner::new_id()));
    let result = (|| -> AppResult<()> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| "无法创建加密密钥文件")?;
        f.write_all(&encrypted)
            .and_then(|_| f.sync_all())
            .map_err(|_| "无法写入加密密钥文件")?;
        drop(f);
        native::replace_file(&temp, &dir.join("deepseek.key"))
            .map_err(|_| "无法替换加密密钥文件".into())
    })();
    let _ = fs::remove_file(temp);
    result
}
pub(crate) fn load_key(data: &Path) -> AppResult<Secret> {
    let p = data.join("ai/deepseek.key");
    let meta = fs::symlink_metadata(&p).map_err(|_| "尚未设置DeepSeek API密钥")?;
    if !meta.is_file() || meta.len() > 16384 {
        return Err("加密密钥文件无效，请重新设置".into());
    }
    let bytes = fs::read(p).map_err(|_| "无法读取加密密钥")?;
    let secret = Secret(
        native::unprotect(&bytes).map_err(|_| "Windows无法解密密钥，请在此用户账户重新设置")?,
    );
    validate_key(secret.as_bytes())?;
    Ok(secret)
}
pub(crate) fn clear_key(data: &Path) -> AppResult<()> {
    let p = data.join("ai/deepseek.key");
    match fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("无法删除API密钥文件，请检查目录权限".into()),
    }
}
pub(crate) fn prompt_key() -> AppResult<Option<Secret>> {
    use windows_sys::Win32::{
        Foundation::{ERROR_CANCELLED, ERROR_SUCCESS},
        Security::Credentials::*,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let caption = wide("映流 · DeepSeek API密钥");
    let message = wide("请将API Key填入密码栏。密钥由Windows加密保存，不会传给网页界面。");
    let target = wide("映流 DeepSeek API");
    let info = CREDUI_INFOW {
        cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32,
        hwndParent: std::ptr::null_mut(),
        pszMessageText: message.as_ptr(),
        pszCaptionText: caption.as_ptr(),
        hbmBanner: std::ptr::null_mut(),
    };
    let mut user = [0u16; 128];
    for (dest, c) in user.iter_mut().zip("DeepSeek".encode_utf16()) {
        *dest = c;
    }
    let mut password = [0u16; 513];
    let mut save = 0;
    let result = unsafe {
        CredUIPromptForCredentialsW(
            &info,
            target.as_ptr(),
            std::ptr::null(),
            0,
            user.as_mut_ptr(),
            user.len() as u32,
            password.as_mut_ptr(),
            password.len() as u32,
            &mut save,
            CREDUI_FLAGS_GENERIC_CREDENTIALS
                | CREDUI_FLAGS_ALWAYS_SHOW_UI
                | CREDUI_FLAGS_DO_NOT_PERSIST
                | CREDUI_FLAGS_KEEP_USERNAME,
        )
    };
    let len = password
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(password.len());
    let ascii = password[..len].iter().all(|c| *c >= 33 && *c <= 126);
    let key = Secret(password[..len].iter().map(|c| *c as u8).collect());
    for c in &mut password {
        unsafe {
            std::ptr::write_volatile(c, 0);
        }
    }
    if result == ERROR_CANCELLED {
        return Ok(None);
    }
    if result != ERROR_SUCCESS {
        return Err("Windows密钥录入窗口无法完成操作，请重试".into());
    }
    if !ascii {
        return Err("API密钥必须为有效ASCII字符".into());
    }
    validate_key(key.as_bytes())?;
    Ok(Some(key))
}
