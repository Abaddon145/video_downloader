use crate::{domain::AppResult,ffmpeg::{models::*,runner::MediaService},native};
use serde::{Serialize,Deserialize};use std::collections::HashSet;
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="lowercase")]
pub enum HardwareAcceleration {#[default] Auto,Cpu,Nvidia,Intel,Amd}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct MediaPreset {pub id:String,pub name:String,pub operation:MediaOperation,pub output_format:OutputFormat,pub video_codec:VideoCodec,pub audio_codec:AudioCodec,pub resolution:Option<u32>,pub quality:Quality,pub hardware_acceleration:HardwareAcceleration,#[serde(default)]pub generate_thumbnails:bool}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub enum AfterDownload {#[default] None,Mp4,Compress,Subtitles,Cover}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase",default,deny_unknown_fields)]
pub struct MediaProSettings {pub hardware_acceleration:HardwareAcceleration,pub presets:Vec<MediaPreset>,pub after_download:AfterDownload,pub enabled_at:u64}
impl Default for MediaProSettings {fn default()->Self{Self{hardware_acceleration:HardwareAcceleration::Auto,presets:builtins(),after_download:AfterDownload::None,enabled_at:0}}}
pub fn builtins()->Vec<MediaPreset>{[("phone","手机视频",OutputFormat::Mp4,VideoCodec::H264,Some(1080),Quality::Balanced,false),("web","网页发布",OutputFormat::Mp4,VideoCodec::H264,Some(1080),Quality::Small,false),("ai","AI 素材",OutputFormat::Mkv,VideoCodec::H264,None,Quality::High,true),("archive","归档",OutputFormat::Mkv,VideoCodec::Hevc,None,Quality::High,false)].into_iter().map(|(id,name,output_format,video_codec,resolution,quality,generate_thumbnails)|MediaPreset{id:id.into(),name:name.into(),operation:MediaOperation::Compress,output_format,video_codec,audio_codec:AudioCodec::Aac,resolution,quality,hardware_acceleration:HardwareAcceleration::Auto,generate_thumbnails}).collect()}
pub fn validate(settings:&MediaProSettings)->AppResult<()>{
 if settings.presets.len()>50{return Err("最多保存50个媒体预设".into());}let mut ids=HashSet::new();
 for p in &settings.presets {if p.id.is_empty()||p.id.len()>80||!p.id.chars().all(|c|c.is_ascii_alphanumeric()||c=='-')||!ids.insert(&p.id)||p.name.trim().is_empty()||p.name.chars().count()>80||p.name.chars().any(char::is_control)||p.resolution.is_some_and(|h|![720,1080,1440,2160].contains(&h))||!matches!(p.operation,MediaOperation::Remux|MediaOperation::Transcode|MediaOperation::Compress|MediaOperation::ExtractAudio|MediaOperation::Screenshot){return Err("媒体预设参数无效".into());}}
 Ok(())
}
impl MediaService {
 pub fn pro_settings(&self)->MediaProSettings{self.pro.lock().unwrap().clone()}
 pub fn save_pro_settings(&self,mut settings:MediaProSettings)->AppResult<MediaProSettings>{
  self.ensure_ready()?;validate(&settings)?;let mut current=self.pro.lock().unwrap();
  settings.enabled_at=if settings.after_download==AfterDownload::None{0}else if settings.after_download!=current.after_download||current.enabled_at==0{crate::service::now()}else{current.enabled_at};
  native::save_atomic(&self.state_file.with_file_name("media-settings.json"),&serde_json::to_vec(&settings).map_err(|_|"无法序列化媒体设置")?)?;*current=settings.clone();Ok(settings)
 }
}
#[cfg(test)]mod tests{use super::*;#[test]fn presets_validate_names_ids_and_exact_defaults(){let s=MediaProSettings::default();assert_eq!(s.presets.len(),4);assert_eq!(s.presets[0].resolution,Some(1080));assert!(s.presets[2].generate_thumbnails);assert!(validate(&s).is_ok());let mut bad=s.clone();bad.presets.push(bad.presets[0].clone());assert!(validate(&bad).is_err());bad=s;bad.presets[0].name="\n".into();assert!(validate(&bad).is_err());}}
