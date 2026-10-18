use super::{
    jobs::Control,
    models::{AnalysisKeyframe, AnalysisMode},
    transcription as t,
};
use crate::{
    domain::AppResult,
    ffmpeg::{models::MediaInfo, probe::DEMUXERS},
};
use std::{collections::HashSet, fs, path::Path, time::Duration};
pub const MAX_IMAGES_PER_REQUEST: usize = 6;
pub const MAX_JPEG_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u16 = 1280;
pub const JPEG_QSCALE: u8 = 5;
#[derive(Clone, Copy)]
pub struct KeyframePolicy {
    pub fallback_ms: i64,
    pub max_frames: usize,
    pub scene_threshold: f64,
}
pub const QUICK: KeyframePolicy = KeyframePolicy {
    fallback_ms: 180000,
    max_frames: 12,
    scene_threshold: 0.45,
};
pub const STANDARD: KeyframePolicy = KeyframePolicy {
    fallback_ms: 90000,
    max_frames: 24,
    scene_threshold: 0.3,
};
pub const DEEP: KeyframePolicy = KeyframePolicy {
    fallback_ms: 45000,
    max_frames: 60,
    scene_threshold: 0.15,
};
pub fn policy(mode: AnalysisMode) -> KeyframePolicy {
    match mode {
        AnalysisMode::Quick => QUICK,
        AnalysisMode::Standard => STANDARD,
        AnalysisMode::Deep => DEEP,
    }
}
pub fn parse_scenes(text: &str, duration_ms: i64) -> Vec<(i64, f64)> {
    let mut time = None;
    let mut scenes = Vec::new();
    for line in text.lines() {
        if let Some(raw) = line
            .split_whitespace()
            .find_map(|part| part.strip_prefix("pts_time:"))
        {
            time = raw
                .parse::<f64>()
                .ok()
                .filter(|t| t.is_finite() && *t >= 0. && *t * 1000. < duration_ms as f64)
                .map(|t| (t * 1000.).round() as i64);
        }
        if let Some(score) = line
            .strip_prefix("lavfi.scene_score=")
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|s| s.is_finite() && (0.0..=1.0).contains(s))
        {
            if let Some(time) = time.take() {
                scenes.push((time, score));
            }
        }
    }
    scenes
}
pub fn select_times(
    duration_ms: i64,
    mode: AnalysisMode,
    scenes: &[(i64, f64)],
) -> AppResult<Vec<(i64, Option<f64>)>> {
    if duration_ms <= 0 || duration_ms > 604800000 {
        return Err("媒体时长超出关键帧分析范围".into());
    }
    let p = policy(mode);
    let fallback_count = ((duration_ms + p.fallback_ms - 1) / p.fallback_ms)
        .max(1)
        .min((p.max_frames / 2) as i64) as usize;
    let mut times = (0..fallback_count)
        .map(|i| {
            (
                ((i as i64 * 2 + 1) * duration_ms / (fallback_count as i64 * 2))
                    .min(duration_ms - 1),
                None,
            )
        })
        .collect::<Vec<_>>();
    times[0].0 = 0;
    let mut sorted = scenes
        .iter()
        .copied()
        .filter(|(time, score)| {
            *time >= 0 && *time < duration_ms && score.is_finite() && (0.0..=1.0).contains(score)
        })
        .collect::<Vec<_>>();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    for (time, score) in sorted {
        if times.len() >= p.max_frames {
            break;
        }
        if times
            .iter()
            .all(|(t, _)| (time - *t).abs() >= 1000.min(duration_ms / 4).max(1))
        {
            times.push((time, Some(score)));
        }
    }
    times.sort_by_key(|t| t.0);
    times.dedup_by_key(|t| t.0);
    Ok(times)
}
fn input(source: &Path) -> Vec<String> {
    [
        "-hide_banner",
        "-nostdin",
        "-loglevel",
        "error",
        "-n",
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        DEMUXERS,
        "-i",
    ]
    .map(str::to_owned)
    .into_iter()
    .chain([source.to_string_lossy().into_owned()])
    .collect()
}
pub fn jpeg_dimensions(bytes: &[u8]) -> AppResult<(u16, u16)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err("关键帧不是有效JPEG图片".into());
    }
    let mut i = 2;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xff {
            return Err("JPEG帧结构无效".into());
        }
        let marker = bytes[i + 1];
        i += 2;
        if marker == 0xff {
            i -= 1;
            continue;
        }
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        if len < 2 || i + len > bytes.len() {
            return Err("JPEG帧不完整".into());
        }
        if [0xc0, 0xc1, 0xc2].contains(&marker) && len >= 8 {
            let height = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]);
            let width = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]);
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
        i += len;
    }
    Err("无法读取JPEG帧尺寸".into())
}
pub fn extract(
    resources: &Path,
    source: &Path,
    cache: &Path,
    analysis_id: &str,
    mode: AnalysisMode,
    info: &MediaInfo,
    control: &Control,
) -> AppResult<Vec<AnalysisKeyframe>> {
    control.check()?;
    if !info.videos.iter().any(|v| !v.attached_picture) {
        return Ok(Vec::new());
    }
    let _source = t::lock_source(source)?;
    let before = t::fingerprint(source)?;
    let duration_ms = (info
        .duration
        .filter(|d| d.is_finite() && *d > 0.)
        .ok_or("视频时长无效")?
        * 1000.)
        .round() as i64;
    let p = policy(mode);
    let exe = t::verified_tool(resources, "ffmpeg.exe")?;
    let mut args = input(source);
    args.extend(["-an".into(),"-sn".into(),"-dn".into(),"-vf".into(),format!("setpts=PTS-STARTPTS,scale=w=320:h=-2,select='gt(scene,{})',metadata=mode=print:key=lavfi.scene_score:file=-",p.scene_threshold),"-fps_mode".into(),"vfr".into(),"-frames:v".into(),(p.max_frames*8).to_string(),"-f".into(),"null".into(),"NUL".into()]);
    let scene = control.capture(
        &exe,
        &args,
        None,
        Duration::from_secs(((duration_ms as u64 / 1000) * 10 + 120).min(86400)),
    )?;
    let candidates = parse_scenes(&String::from_utf8_lossy(&scene), duration_ms);
    let times = select_times(duration_ms, mode, &candidates)?;
    let directory = cache.join("keyframes");
    fs::create_dir_all(&directory).map_err(|_| "无法创建画面缓存目录")?;
    let mut frames = Vec::new();
    let mut hashes = HashSet::new();
    // Decode selected frames together: TS and long-GOP inputs may return EOF for a fast seek.
    let generation = crate::ffmpeg::runner::new_id();
    struct Partials {
        directory: std::path::PathBuf,
        prefix: String,
        count: usize,
    }
    impl Drop for Partials {
        fn drop(&mut self) {
            for n in 1..=self.count {
                let _ = fs::remove_file(self.directory.join(format!("{}-{n:04}.jpg", self.prefix)));
            }
        }
    }
    let partials = Partials {
        directory: directory.clone(),
        prefix: generation.clone(),
        count: times.len() + 1,
    };
    let selection = times
        .iter()
        .map(|(time, _)| {
            let seconds = *time as f64 / 1000.;
            format!("gte(t,{seconds:.3})*if(isnan(prev_t),1,lt(prev_t,{seconds:.3}))")
        })
        .collect::<Vec<_>>()
        .join("+");
    let filter=format!("setpts=PTS-STARTPTS,select='{selection}',metadata=mode=add:key=yingliu.frame:value=1,metadata=mode=print:key=yingliu.frame:file=-,scale=w='max(2,min({MAX_IMAGE_DIMENSION},iw))':h='max(2,min({MAX_IMAGE_DIMENSION},ih))':force_original_aspect_ratio=decrease:force_divisible_by=2:out_range=full,format=yuvj420p");
    let pattern = directory.join(format!("{generation}-%04d.jpg"));
    let mut args = input(source);
    args.extend([
        "-map".into(),
        "0:V:0".into(),
        "-an".into(),
        "-sn".into(),
        "-dn".into(),
        "-vf".into(),
        filter,
        "-frames:v".into(),
        times.len().to_string(),
        "-fps_mode".into(),
        "vfr".into(),
        "-q:v".into(),
        JPEG_QSCALE.to_string(),
        pattern.to_string_lossy().into_owned(),
    ]);
    let output = control.capture(
        &exe,
        &args,
        None,
        Duration::from_secs(((duration_ms as u64 / 1000) * 10 + 120).min(86400)),
    )?;
    let actual = String::from_utf8_lossy(&output)
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .find_map(|part| part.strip_prefix("pts_time:"))
        })
        .filter_map(|s| s.parse::<f64>().ok())
        .filter(|t| t.is_finite() && *t >= 0. && *t * 1000. < duration_ms as f64)
        .map(|t| (t * 1000.).round() as i64)
        .collect::<Vec<_>>();
    if actual.is_empty() || actual.len() > times.len() {
        return Err("无法取得所选时间点画面".into());
    }
    for (index, time) in actual.into_iter().enumerate() {
        control.check()?;
        if t::fingerprint(source)? != before {
            return Err("源视频在分析过程中发生变化，请重新开始分析".into());
        }
        let score = times
            .iter()
            .filter(|(t, _)| (*t - time).abs() < 1000)
            .filter_map(|(_, score)| *score)
            .max_by(f64::total_cmp);
        let temporary = directory.join(format!("{generation}-{:04}.jpg", index + 1));
        let name = format!(
            "{:04}-{:02}-{:02}-{:02}-{:03}.jpg",
            index + 1,
            time / 3600000,
            (time / 60000) % 60,
            (time / 1000) % 60,
            time % 1000
        );
        let file = directory.join(name);
        crate::native::replace_file(&temporary, &file).map_err(|_| "无法保存关键帧")?;
        let meta = fs::metadata(&file).map_err(|_| "无法取得所选时间点画面")?;
        if meta.len() > MAX_JPEG_BYTES {
            return Err("关键帧过大，已阻止上传".into());
        }
        let bytes = fs::read(&file).map_err(|_| "无法读取关键帧")?;
        let (w, h) = jpeg_dimensions(&bytes)?;
        if w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
            return Err("关键帧尺寸超出限制".into());
        }
        let sha = t::file_hash(&file)?;
        if !hashes.insert(sha.clone()) {
            let _ = fs::remove_file(&file);
            continue;
        }
        frames.push(AnalysisKeyframe {
            id: format!("{analysis_id}-frame-{index}"),
            analysis_id: analysis_id.into(),
            time_ms: time,
            path: file.to_string_lossy().into_owned(),
            sha256: sha,
            scene_score: score,
        });
    }
    drop(partials);
    if t::fingerprint(source)? != before {
        return Err("源视频在分析过程中发生变化，请重新开始分析".into());
    }
    Ok(frames)
}
