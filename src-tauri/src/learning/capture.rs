use super::{models::VideoNote, service::LearningService};
use crate::{
    ai::{keyframes, transcription as t},
    domain::AppResult,
    ffmpeg::{command, models::*},
    knowledge::{generated::NoteReference, service::KnowledgeService},
};
use std::{fs, path::Path};
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotNoteResult {
    pub note: VideoNote,
    pub reference: NoteReference,
    pub attachment_path: String,
    pub warning: Option<String>,
}
pub(crate) fn capture_args(
    source: &Path,
    directory: &Path,
    output: &Path,
    info: &MediaInfo,
    time: i64,
) -> AppResult<Vec<String>> {
    if time < 0
        || time > 604800000
        || info
            .duration
            .is_none_or(|d| !d.is_finite() || d <= 0. || time as f64 >= d * 1000.)
    {
        return Err("截图时间必须在真实课程范围内".into());
    }
    let r = MediaRequest {
        input_path: source.to_string_lossy().into(),
        output_dir: directory.to_string_lossy().into(),
        operation: MediaOperation::Screenshot,
        output_format: OutputFormat::Jpg,
        start: time as f64 / 1000.,
        height: Some(720),
        ..Default::default()
    };
    let mut args = command::build(&r, info, output)?.args;
    if !super::thumbnail::accurate_seek(&mut args) {
        return Err("无法设置准确截图时间".into());
    }
    Ok(args)
}
impl LearningService {
    pub fn capture_note(
        &self,
        k: &KnowledgeService,
        item: &str,
        time: i64,
        vault: &str,
        content: &str,
        session: &str,
    ) -> AppResult<ScreenshotNoteResult> {
        crate::media::trim::timeline::validate_session(session)?;
        if item.is_empty() || item.len() > 128 {
            return Err("课程标识无效".into());
        }
        let content = super::repository::text(content, 4000, false)?;
        let (course, source) = self.with(|r| Ok((r.item(item)?, r.playback_file(item)?)))?;
        let _source = t::lock_source(&source)?;
        let before = t::fingerprint(&source)?;
        let info = self
            .media
            .probe_file(&source, &format!("{session}-probe"), false)?;
        if !info.videos.iter().any(|v| !v.attached_picture) {
            return Err("音频课程没有可截图的视频画面".into());
        }
        let directory = self.media.preview_directory(session)?;
        let output = directory.join(format!("{}.jpg", crate::ffmpeg::runner::new_id()));
        let args = capture_args(&source, &directory, &output, &info, time)?;
        let read: AppResult<Vec<u8>> = (|| {
            self.media.auxiliary(session, &args, 30)?;
            let file = super::cache::owned(&directory, &output)?;
            if fs::metadata(&file).map_err(|_| "截图未生成")?.len() > 8 * 1024 * 1024 {
                return Err("截图超过8MiB".into());
            }
            fs::read(file).map_err(|_| "无法读取截图".into())
        })();
        let _ = fs::remove_file(&output);
        let bytes = read?;
        keyframes::jpeg_dimensions(&bytes)?;
        if t::fingerprint(&source)? != before {
            return Err("课程文件已变化，截图笔记未保存".into());
        }
        let attachment_folder = k.settings()?.attachments_folder;
        let note_id = super::repository::id();
        let prepared =
            k.prepare_screenshot(vault, &note_id, &course.title, item, time, &content, &bytes)?;
        let published = (|| {
            let state = self.media.state.lock().map_err(|_| "媒体任务状态不可用")?;
            if state.exiting
                || state
                    .cancelled_previews
                    .iter()
                    .any(|s| session == s || session.starts_with(&format!("{s}-")))
            {
                return Err("截图已取消".into());
            }
            self.with(|r|{if r.playback_file(item)?!=source||t::fingerprint(&source)?!=before{return Err("课程关联或源文件已变化".into())}k.publish_prepared(&prepared)?;
    let tx=r.conn.unchecked_transaction().map_err(super::db::error)?;
    let body=if content.is_empty(){"截图笔记"}else{&content};let now=crate::service::now() as i64;let path=format!("{}/截图笔记.md",prepared.target);
    tx.execute("INSERT INTO video_notes(id,item_id,time_ms,content,created_at,updated_at,markdown_vault_id,markdown_path) VALUES(?1,?2,?3,?4,?5,?5,?6,?7)",rusqlite::params![note_id,item,time,body,now,vault,path]).map_err(|_|"截图笔记已保存到知识库，但课程关联失败，请在知识库中查看".to_owned())?;tx.commit().map_err(super::db::error)?;
    Ok(ScreenshotNoteResult{note:VideoNote{id:note_id.clone(),item_id:item.into(),time_ms:time,content:body.into(),created_at:now,updated_at:now,markdown_vault_id:Some(vault.into()),markdown_path:Some(path.clone())},reference:NoteReference{vault_id:vault.into(),path},attachment_path:format!("{}/{}/截图.jpg",prepared.target,attachment_folder),warning:None})
   })
        })();
        if published.is_err() {
            k.discard_prepared(&prepared);
        }
        let mut result = published?;
        if let Err(error) = k.index_screenshot(&prepared.vault_id, &result.reference.path) {
            result.warning = Some(error)
        }
        self.changed("note", &note_id);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn screenshot_uses_accurate_output_seek_and_rejects_bad_times() {
        let dir = std::env::temp_dir();
        let info = MediaInfo {
            duration: Some(4.),
            videos: vec![crate::ffmpeg::models::VideoStreamInfo {
                index: 0,
                codec: "h264".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let args = capture_args(
            &dir.join("in.mp4"),
            &dir,
            &dir.join("capture.jpg"),
            &info,
            1250,
        )
        .unwrap();
        let seek = args
            .iter()
            .position(|v| v == "-ss")
            .expect("precise seek required");
        let input = args.iter().position(|v| v == "-i").unwrap();
        assert!(seek > input);
        assert_eq!(args[seek + 1], "1.250");
        assert!(capture_args(
            &dir.join("in.mp4"),
            &dir,
            &dir.join("capture.jpg"),
            &info,
            -1
        )
        .is_err());
    }
    #[test]
    #[ignore = "Requires verified FFmpeg and ffprobe; AI_QA_RESOURCES"]
    fn real_screenshot_is_saved_through_safe_vault_with_custom_attachment_folder() {
        use crate::{
            ai::jobs::Control,
            knowledge::{models::KnowledgeSettings, vault::Vault},
        };
        use std::{path::PathBuf, time::Duration};
        let resources = PathBuf::from(std::env::var("AI_QA_RESOURCES").unwrap());
        let ffmpeg = t::verified_tool(&resources, "ffmpeg.exe").unwrap();
        let root = std::env::temp_dir().join(format!(
            "映流截图 #&（）-{}",
            crate::ffmpeg::runner::new_id()
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("课程.mp4");
        let control = Control::default();
        let mut args = [
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-n",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=5",
            "-t",
            "3",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ]
        .map(str::to_owned)
        .to_vec();
        args.push(source.to_string_lossy().into());
        control
            .capture(&ffmpeg, &args, None, Duration::from_secs(30))
            .unwrap();
        let source = fs::canonicalize(source).unwrap();
        let before = t::file_hash(&source).unwrap();
        let info = t::probe_media(&resources, &source, &control).unwrap();
        let output = root.join("截图.jpg");
        control
            .capture(
                &ffmpeg,
                &capture_args(&source, &root, &output, &info, 1250).unwrap(),
                None,
                Duration::from_secs(30),
            )
            .unwrap();
        let jpeg = fs::read(&output).unwrap();
        assert_eq!(keyframes::jpeg_dimensions(&jpeg).unwrap(), (320, 180));
        let mut settings = KnowledgeSettings::default();
        settings.attachments_folder = "课程图片".into();
        crate::native::save_atomic(
            &root.join("knowledge-settings.json"),
            &serde_json::to_vec(&settings).unwrap(),
        )
        .unwrap();
        let vault_root = root.join("Vault 中文");
        fs::create_dir(&vault_root).unwrap();
        let k = KnowledgeService::new(&root);
        let vault = k
            .set_vault(vault_root.to_str().unwrap(), false)
            .unwrap()
            .vault_id
            .unwrap();
        let prepared = k
            .prepare_screenshot(
                &vault,
                "capture-test",
                "课程安全",
                "lesson",
                1250,
                "用户说明",
                &jpeg,
            )
            .unwrap();
        k.publish_prepared(&prepared).unwrap();
        let path = format!("{}/截图笔记.md", prepared.target);
        k.index_screenshot(&vault, &path).unwrap();
        let doc = k.read(&vault, &path).unwrap().data;
        assert!(doc.text.contains("![[课程图片/截图.jpg]]"));
        assert!(doc.text.contains("yingliu://video/lesson?t=1250"));
        assert!(doc.text.ends_with("用户说明\n"));
        let v = Vault::open(&vault_root).unwrap();
        assert_eq!(
            v.read_bytes(
                &format!("{}/课程图片/截图.jpg", prepared.target),
                8 * 1024 * 1024
            )
            .unwrap(),
            jpeg
        );
        assert_eq!(t::file_hash(&source).unwrap(), before);
        println!("REAL_SCREENSHOT time_ms=1250 jpeg=320x180 custom_attachments=true safe_vault=true source_sha256={before} source_unchanged=true");
        drop(v);
        drop(k);
        fs::remove_dir_all(root).unwrap();
    }
}
