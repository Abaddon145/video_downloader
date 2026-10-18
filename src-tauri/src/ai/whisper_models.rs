use super::{
    service::AiService,
    transcription::{self, ModelSpec},
};
use crate::{domain::AppResult, native};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf, sync::Arc, time::Duration};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    #[serde(flatten)]
    pub spec: ModelSpec,
    pub installed: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownload {
    pub id: String,
    pub model_id: String,
    pub status: String,
    pub received: u64,
    pub total: u64,
    pub error: Option<String>,
}
#[derive(Default)]
pub struct ModelRuntime {
    pub snapshot: Option<ModelDownload>,
    pub task: Option<tauri::async_runtime::JoinHandle<()>>,
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
impl AiService {
    pub fn whisper_models(&self) -> AppResult<Vec<ModelView>> {
        Ok(transcription::model_specs()?
            .into_iter()
            .map(|spec| {
                let installed = fs::metadata(self.data.join("ai/models").join(&spec.filename))
                    .is_ok_and(|m| m.is_file() && m.len() == spec.bytes);
                ModelView { spec, installed }
            })
            .collect())
    }
    pub fn model_download(&self) -> AppResult<Option<ModelDownload>> {
        Ok(self
            .models
            .lock()
            .map_err(|_| "模型下载状态不可用")?
            .snapshot
            .clone())
    }
    fn model_update(
        &self,
        id: &str,
        status: &str,
        received: u64,
        error: Option<String>,
    ) -> AppResult<()> {
        let mut state = self.models.lock().map_err(|_| "模型下载状态不可用")?;
        let snapshot = state
            .snapshot
            .as_mut()
            .filter(|s| {
                s.id == id && !["cancelled", "completed", "failed"].contains(&s.status.as_str())
            })
            .ok_or("模型下载已取消")?;
        snapshot.status = status.into();
        snapshot.received = received;
        snapshot.error = error;
        self.emit("ai-model-download-updated", snapshot.clone());
        Ok(())
    }
    pub fn download_model(self: &Arc<Self>, model_id: &str) -> AppResult<ModelDownload> {
        let spec = transcription::model_specs()?
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or("语音模型选项无效")?;
        let mut state = self.models.lock().map_err(|_| "模型下载状态不可用")?;
        if state
            .snapshot
            .as_ref()
            .is_some_and(|s| ["downloading", "verifying"].contains(&s.status.as_str()))
        {
            return Err("已有语音模型正在下载，请先完成或取消".into());
        }
        let view = ModelDownload {
            id: crate::ffmpeg::runner::new_id().replace("media-", "model-"),
            model_id: spec.id.clone(),
            status: "downloading".into(),
            received: 0,
            total: spec.bytes,
            error: None,
        };
        state.snapshot = Some(view.clone());
        let service = self.clone();
        let id = view.id.clone();
        state.task = Some(tauri::async_runtime::spawn(async move {
            if let Err(error) = service.fetch_model(&id, &spec).await {
                let received = service
                    .model_download()
                    .ok()
                    .flatten()
                    .filter(|s| s.id == id)
                    .map(|s| s.received)
                    .unwrap_or(0);
                let _ = service.model_update(&id, "failed", received, Some(error));
            }
        }));
        Ok(view)
    }
    async fn fetch_model(&self, id: &str, spec: &ModelSpec) -> AppResult<()> {
        let dir = self.data.join("ai/models");
        fs::create_dir_all(&dir).map_err(|_| "无法创建语音模型目录")?;
        let temp = Temporary(dir.join(format!("{id}.part")));
        let http = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(1800))
            .user_agent(concat!("Yingliu/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| "无法初始化模型下载")?;
        let mut response = http
            .get(&spec.url)
            .send()
            .await
            .map_err(|_| "模型下载无法连接，请检查网络后重试")?;
        if !response.status().is_success() {
            return Err(format!(
                "模型下载失败（HTTP {}），可重试或导入本地模型",
                response.status().as_u16()
            ));
        }
        if response.content_length().is_some_and(|n| n != spec.bytes) {
            return Err("上游模型大小与固定清单不符，已停止下载".into());
        }
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp.0)
            .map_err(|_| "无法创建模型临时文件")?;
        let mut sha = Sha256::new();
        let mut received = 0u64;
        let mut last_event = std::time::Instant::now();
        while let Some(chunk) = response.chunk().await.map_err(|_| "模型下载中断，请重试")?
        {
            received += chunk.len() as u64;
            if received > spec.bytes {
                return Err("模型大小超过固定清单，已停止下载".into());
            }
            file.write_all(&chunk)
                .map_err(|_| "模型写入失败，请检查磁盘空间")?;
            sha.update(&chunk);
            if last_event.elapsed() >= Duration::from_millis(200) || received == spec.bytes {
                self.model_update(id, "downloading", received, None)?;
                last_event = std::time::Instant::now();
            }
        }
        self.model_update(id, "verifying", received, None)?;
        if received != spec.bytes || format!("{:x}", sha.finalize()) != spec.sha256 {
            return Err("模型SHA256校验失败，未加载该文件，请重新下载".into());
        }
        file.sync_all().map_err(|_| "无法保存模型临时文件")?;
        drop(file);
        transcription::validate_model_header(&temp.0)?;
        let mut state = self.models.lock().map_err(|_| "模型下载状态不可用")?;
        let snapshot = state
            .snapshot
            .as_mut()
            .filter(|s| s.id == id && s.status == "verifying")
            .ok_or("模型下载已取消")?;
        native::replace_file(&temp.0, &dir.join(&spec.filename))
            .map_err(|_| "无法启用已验证模型，请检查文件权限")?;
        snapshot.status = "completed".into();
        self.emit("ai-model-download-updated", snapshot.clone());
        Ok(())
    }
    pub fn cancel_model_download(&self, id: &str) -> AppResult<()> {
        let mut state = self.models.lock().map_err(|_| "模型下载状态不可用")?;
        let snapshot = state
            .snapshot
            .as_mut()
            .filter(|s| s.id == id)
            .ok_or("找不到模型下载任务")?;
        if ["completed", "failed", "cancelled"].contains(&snapshot.status.as_str()) {
            return Ok(());
        }
        snapshot.status = "cancelled".into();
        self.emit("ai-model-download-updated", snapshot.clone());
        if let Some(task) = state.task.take() {
            task.abort();
        }
        Ok(())
    }
    pub fn import_model(&self, path: &str, expected_sha256: Option<&str>) -> AppResult<String> {
        let source = crate::learning::importer::regular_file(path)?;
        let meta = fs::metadata(&source).map_err(|_| "无法读取语音模型")?;
        if meta.len() < 1024 * 1024 || meta.len() > 4 * 1024 * 1024 * 1024 {
            return Err("请选择完整的Whisper GGML模型（1MiB至4GiB）".into());
        }
        let _source = transcription::lock_source(&source)?;
        transcription::validate_model_header(&source)?;
        let sha = transcription::file_hash(&source)?;
        if expected_sha256.is_some_and(|s| {
            s.len() != 64
                || !s.bytes().all(|b| b.is_ascii_hexdigit())
                || !s.eq_ignore_ascii_case(&sha)
        }) {
            return Err("模型SHA256与提供的摘要不一致".into());
        }
        let known = transcription::model_specs()?
            .into_iter()
            .find(|s| s.sha256 == sha && s.bytes == meta.len());
        if known.is_none()
            && expected_sha256.is_none_or(|s| {
                s.len() != 64
                    || !s.bytes().all(|b| b.is_ascii_hexdigit())
                    || !s.eq_ignore_ascii_case(&sha)
            })
        {
            return Err("自定义模型需要提供可信来源的SHA256，且必须与文件一致".into());
        }
        let dir = self.data.join("ai/models");
        fs::create_dir_all(&dir).map_err(|_| "无法创建模型目录")?;
        let temp = Temporary(dir.join(format!("{}.part", crate::ffmpeg::runner::new_id())));
        fs::copy(&source, &temp.0).map_err(|_| "模型导入失败，请检查磁盘空间")?;
        let file = fs::OpenOptions::new()
            .write(true)
            .open(&temp.0)
            .map_err(|_| "无法核验导入模型")?;
        file.sync_all().map_err(|_| "无法写入导入模型")?;
        drop(file);
        if transcription::file_hash(&temp.0)? != sha {
            return Err("导入过程文件发生变化，请重试".into());
        }
        let id = known.as_ref().map(|m| m.id.as_str()).unwrap_or("custom");
        let filename = known
            .as_ref()
            .map(|m| m.filename.as_str())
            .unwrap_or("ggml-custom.bin");
        native::replace_file(&temp.0, &dir.join(filename)).map_err(|_| "无法启用导入模型")?;
        if id == "custom" {
            native::save_atomic(
                &dir.join("custom.json"),
                &serde_json::to_vec(&serde_json::json!({"sha256":sha,"bytes":meta.len()}))
                    .map_err(|_| "无法保存模型校验信息")?,
            )?;
        }
        Ok(id.into())
    }
}
