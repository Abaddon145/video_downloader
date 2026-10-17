use super::{jobs::Control, models::TranscriptSegment};
use crate::{
    domain::AppResult,
    ffmpeg::{models::MediaInfo, probe},
    service::verify_hash,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    pub id: String,
    pub label: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub url: String,
    pub source: String,
    pub license: String,
}
pub fn model_specs() -> AppResult<Vec<ModelSpec>> {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../resources/ai-components.json"))
            .map_err(|_| "语音模型清单无效")?;
    serde_json::from_value(v["models"].clone()).map_err(|_| "语音模型清单无效".into())
}
pub fn file_hash(path: &Path) -> AppResult<String> {
    let mut file = File::open(path).map_err(|_| "无法读取文件进行完整性检查")?;
    let mut sha = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|_| "文件完整性检查失败")?;
        if n == 0 {
            break;
        }
        sha.update(&buf[..n]);
    }
    Ok(format!("{:x}", sha.finalize()))
}
pub fn verify_model(data: &Path, id: &str) -> AppResult<PathBuf> {
    let (filename, size, sha) = if id == "custom" {
        let p = data.join("ai/models/custom.json");
        let v: serde_json::Value =
            serde_json::from_slice(&fs::read(p).map_err(|_| "尚未导入自定义语音模型")?)
                .map_err(|_| "自定义模型校验记录无效")?;
        (
            "ggml-custom.bin".to_owned(),
            v["bytes"].as_u64().ok_or("模型大小记录无效")?,
            v["sha256"].as_str().ok_or("模型摘要记录无效")?.to_owned(),
        )
    } else {
        let spec = model_specs()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("语音模型选项无效")?;
        (spec.filename, spec.bytes, spec.sha256)
    };
    let path = data.join("ai/models").join(filename);
    let meta = fs::symlink_metadata(&path).map_err(|_| "需要先下载或导入语音识别模型")?;
    use std::os::windows::fs::MetadataExt;
    if !meta.is_file() || meta.file_attributes() & 0x400 != 0 || meta.len() != size {
        return Err("语音模型损坏，请重新下载或导入".into());
    }
    if sha.len() != 64 || verify_hash(&path, &sha).is_err() {
        return Err("语音模型SHA256校验失败，请重新下载或导入".into());
    }
    validate_model_header(&path)?;
    Ok(path)
}
pub fn validate_model_header(path: &Path) -> AppResult<()> {
    let mut header = [0; 44];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut header))
        .map_err(|_| "语音模型文件不完整")?;
    let value = |n: usize| u32::from_le_bytes(header[n * 4..n * 4 + 4].try_into().unwrap());
    if value(0) != 0x67676d6c
        || !(50000..=60000).contains(&value(1))
        || value(2) != 1500
        || ![384, 512, 768, 1024, 1280].contains(&value(3))
        || value(9) == 0
        || value(9) > 32
    {
        return Err("不是受支持的Whisper GGML语音模型".into());
    }
    Ok(())
}
pub fn verified_tool(resources: &Path, name: &str) -> AppResult<PathBuf> {
    let path = resources.join("tools").join(name);
    let v: serde_json::Value = serde_json::from_slice(
        &fs::read(resources.join("tools/binaries.json"))
            .map_err(|_| "本地组件校验清单缺失，请重新安装")?,
    )
    .map_err(|_| "本地组件校验清单无效")?;
    let sha = v
        .get(name)
        .and_then(|v| v.get("sha256"))
        .and_then(serde_json::Value::as_str)
        .ok_or("本地组件未登记，请重新安装")?;
    verify_hash(&path, sha).map_err(|_| "本地组件缺失或SHA256校验失败，请重新安装")?;
    Ok(path)
}
pub fn whisper_binary(resources: &Path) -> AppResult<PathBuf> {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../resources/ai-components.json"))
            .map_err(|_| "语音组件清单无效")?;
    for (name, sha) in v["binaryHashes"].as_object().ok_or("语音组件清单无效")? {
        verify_hash(
            &resources.join("tools").join(name),
            sha.as_str().ok_or("语音组件摘要无效")?,
        )
        .map_err(|_| format!("Whisper组件缺失或校验失败：{name}，请重新安装"))?;
    }
    Ok(resources.join("tools/whisper-cli.exe"))
}
pub fn fingerprint(path: &Path) -> AppResult<String> {
    let mut f = File::open(path).map_err(|_| "源文件无法读取")?;
    let meta = f.metadata().map_err(|_| "无法读取源文件信息")?;
    let mut info = unsafe {
        std::mem::zeroed::<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION>()
    };
    if unsafe {
        windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
            f.as_raw_handle(),
            &mut info,
        )
    } == 0
    {
        return Err("无法读取源文件标识".into());
    }
    let mut sha = Sha256::new();
    sha.update(meta.len().to_le_bytes());
    sha.update(info.dwVolumeSerialNumber.to_le_bytes());
    sha.update(info.nFileIndexHigh.to_le_bytes());
    sha.update(info.nFileIndexLow.to_le_bytes());
    sha.update(info.ftLastWriteTime.dwHighDateTime.to_le_bytes());
    sha.update(info.ftLastWriteTime.dwLowDateTime.to_le_bytes());
    let mut buf = [0; 65536];
    let n = f.read(&mut buf).map_err(|_| "无法核验源文件")?;
    sha.update(&buf[..n]);
    if meta.len() > 65536 {
        f.seek(SeekFrom::End(-65536))
            .map_err(|_| "无法核验源文件")?;
        let n = f.read(&mut buf).map_err(|_| "无法核验源文件")?;
        sha.update(&buf[..n]);
    }
    Ok(format!("{:x}", sha.finalize()))
}
pub fn lock_source(path: &Path) -> AppResult<File> {
    fs::OpenOptions::new()
        .read(true)
        .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
        .open(path)
        .map_err(|_| "源文件正在被其他程序修改或无法读取，请稍后重试".into())
}
pub fn probe_media(resources: &Path, path: &Path, control: &Control) -> AppResult<MediaInfo> {
    let exe = verified_tool(resources, "ffprobe.exe")?;
    let bytes = control.capture(&exe, &probe::arguments(path), None, Duration::from_secs(30))?;
    probe::parse(&String::from_utf8_lossy(&bytes), path)
}
pub fn extract_audio(
    resources: &Path,
    source: &Path,
    cache: &Path,
    info: &MediaInfo,
    control: &Control,
) -> AppResult<Option<PathBuf>> {
    if info.audios.is_empty() {
        return Ok(None);
    }
    control.check()?;
    fs::create_dir_all(cache).map_err(|_| "无法创建AI缓存目录")?;
    let wav = cache.join("audio.wav");
    if wav.exists() {
        fs::remove_file(&wav).map_err(|_| "无法清理旧临时音频")?;
    }
    let exe = verified_tool(resources, "ffmpeg.exe")?;
    let args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-loglevel".into(),
        "error".into(),
        "-n".into(),
        "-protocol_whitelist".into(),
        "file,pipe".into(),
        "-format_whitelist".into(),
        crate::ffmpeg::probe::DEMUXERS.into(),
        "-i".into(),
        source.to_string_lossy().into_owned(),
        "-map".into(),
        "0:a:0".into(),
        "-vn".into(),
        "-ac".into(),
        "1".into(),
        "-ar".into(),
        "16000".into(),
        "-c:a".into(),
        "pcm_s16le".into(),
        wav.to_string_lossy().into_owned(),
    ];
    control.capture(
        &exe,
        &args,
        None,
        Duration::from_secs(((info.duration.unwrap_or(3600.) * 10.) as u64 + 120).min(86400)),
    )?;
    Ok(Some(wav))
}
pub fn parse_transcript(
    bytes: &[u8],
    analysis_id: &str,
    duration_ms: i64,
) -> AppResult<(Option<String>, Vec<TranscriptSegment>)> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("语音转录结果过大".into());
    }
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Whisper转录JSON无效或不完整")?;
    let language = v
        .pointer("/result/language")
        .and_then(serde_json::Value::as_str)
        .filter(|s| s.len() <= 16)
        .map(str::to_owned);
    let rows = v["transcription"]
        .as_array()
        .ok_or("Whisper未返回带时间码的转录")?;
    if rows.len() > 200000 {
        return Err("语音转录段落过多".into());
    }
    let mut segments = Vec::new();
    let mut previous = 0;
    for row in rows {
        let start = row
            .pointer("/offsets/from")
            .and_then(serde_json::Value::as_i64)
            .ok_or("转录起始时间无效")?;
        let end = row
            .pointer("/offsets/to")
            .and_then(serde_json::Value::as_i64)
            .ok_or("转录结束时间无效")?;
        let text = row["text"].as_str().ok_or("转录文本无效")?.trim();
        if start < 0
            || start < previous
            || start >= duration_ms
            || end <= start
            || end > duration_ms.saturating_add(1000)
            || text.len() > 16000
        {
            return Err("转录时间或长度超出真实媒体范围".into());
        }
        previous = start;
        if text.is_empty() {
            continue;
        }
        let index = segments.len() as u32;
        segments.push(TranscriptSegment {
            id: format!("{analysis_id}-segment-{index}"),
            analysis_id: analysis_id.into(),
            segment_index: index,
            start_ms: start,
            end_ms: end.min(duration_ms),
            text: text.into(),
        });
    }
    Ok((language, segments))
}
pub fn transcribe(
    resources: &Path,
    data: &Path,
    cache: &Path,
    model_id: &str,
    analysis_id: &str,
    duration_ms: i64,
    control: &Control,
) -> AppResult<(Option<String>, Vec<TranscriptSegment>)> {
    control.check()?;
    let model = verify_model(data, model_id)?;
    let exe = whisper_binary(resources)?;
    // v1.9.1's Windows argv uses the system code page. ASCII relative inputs also work under a Chinese cache directory.
    let linked = cache.join("model.bin");
    if linked.exists() {
        fs::remove_file(&linked).map_err(|_| "无法清理临时模型链接")?;
    }
    if fs::hard_link(&model, &linked).is_err() {
        fs::copy(&model, &linked).map_err(|_| "无法准备本地语音模型，请检查剩余空间")?;
    }
    let output = cache.join("transcript.json");
    if output.exists() {
        fs::remove_file(&output).map_err(|_| "无法清理旧转录临时文件")?;
    }
    let threads = std::thread::available_parallelism()
        .map(|n| (n.get() / 2).clamp(1, 8))
        .unwrap_or(4)
        .to_string();
    let args = [
        "-m",
        "model.bin",
        "-f",
        "audio.wav",
        "-l",
        "auto",
        "-t",
        &threads,
        "-ng",
        "-oj",
        "-of",
        "transcript",
        "-np",
    ]
    .map(str::to_owned);
    let run = control.capture(
        &exe,
        &args,
        Some(cache),
        Duration::from_secs(((duration_ms.max(1000) as u64 / 1000) * 30 + 120).min(86400)),
    );
    let _ = fs::remove_file(&linked);
    run?;
    control.check()?;
    if fs::metadata(&output)
        .map_err(|_| "Whisper未生成转录结果")?
        .len()
        > 32 * 1024 * 1024
    {
        return Err("语音转录结果过大".into());
    }
    let bytes = fs::read(&output).map_err(|_| "Whisper未生成转录结果，请检查语音模型")?;
    parse_transcript(&bytes, analysis_id, duration_ms)
}
