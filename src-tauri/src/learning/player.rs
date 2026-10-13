
use super::{db,importer::regular_file,models::*,repository::Repository,service::LearningService};
use crate::{domain::AppResult,ffmpeg::{models::*,command}};use rusqlite::params;use std::{fs,path::{Path,PathBuf}};
impl Repository{
 pub fn playback_file(&self,id:&str)->AppResult<PathBuf>{let item=self.item(id)?;let result=item.file_path.as_deref().ok_or_else(||"课程尚未下载".to_string()).and_then(regular_file);match result{Ok(path) if super::importer::video_path(&path.to_string_lossy())=>{self.conn.execute("UPDATE series_items SET available=1 WHERE id=?1",[id]).map_err(db::error)?;Ok(path)},_=>{self.conn.execute("UPDATE series_items SET available=0 WHERE id=?1",[id]).map_err(db::error)?;Err(if item.file_path.is_some(){"文件已移动或不存在，请重新定位"}else{"课程尚未下载，请先下载视频"}.into())}}}
 pub fn neighbors(&self,id:&str)->AppResult<(Option<String>,Option<String>)>{let item=self.item(id)?;let ids=self.item_ids(&item.series_id)?;let n=ids.iter().position(|x|x==id).ok_or("课程不存在")?;Ok((n.checked_sub(1).and_then(|n|ids.get(n)).cloned(),ids.get(n+1).cloned()))}
}
impl LearningService{
 pub fn prepare_player(&self,id:&str,session:&str,proxy:bool)->AppResult<PlayerMedia>{
  crate::media::trim::timeline::validate_session(session)?;
  let source=self.with(|r|r.playback_file(id));let source=match source{Ok(p)=>p,Err(e)=>{self.changed("missing",id);return Err(e);}};
  let (item,series,settings,previous,next)=self.with(|r|{let item=r.item(id)?;let (a,b)=r.neighbors(id)?;Ok((item.clone(),r.get_series(&item.series_id)?,r.settings()?,a,b))})?;
  let path=self.media.editor_preview(&source.to_string_lossy(),session,proxy)?;
  let mut subtitles=vec![];let mut warnings=vec![];
  if settings.auto_subtitles{
   let stem=source.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
   if let Some(parent)=source.parent(){if let Ok(entries)=fs::read_dir(parent){let mut candidates=entries.take(10000).filter_map(Result::ok).map(|e|e.path()).filter(|p|{let name=p.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();(name==stem||name.starts_with(&format!("{stem}.")))&&["srt","ass","vtt"].contains(&p.extension().unwrap_or_default().to_string_lossy().to_lowercase().as_str())}).collect::<Vec<_>>();candidates.sort();for p in candidates.into_iter().take(4){match self.subtitle(&p,session){Ok(s)=>subtitles.push(s),Err(e)=>warnings.push(e)}}}}
   if subtitles.is_empty(){if let Ok(info)=self.media.probe(&source.to_string_lossy()){if let Some(track)=info.subtitles.iter().find(|s|["subrip","ass","ssa","webvtt","mov_text"].contains(&s.codec.as_str())){let mut selected=info.clone();selected.subtitles=vec![track.clone()];match self.convert_subtitle(&source,&selected,session,Some(track.index)){Ok(path)=>subtitles.push(PlayerSubtitle{label:"内嵌字幕".into(),path}),Err(e)=>warnings.push(e)}}}}
  }
  Ok(PlayerMedia{item,series,settings,previous,next,path,subtitles,subtitle_warning:(!warnings.is_empty()).then(||warnings.join("；"))})
 }
 fn convert_subtitle(&self,input:&Path,info:&MediaInfo,session:&str,index:Option<u32>)->AppResult<String>{let dir=self.media.preview_directory(session)?;let target=dir.join(format!("{}.vtt",super::repository::id()));let request=MediaRequest{input_path:input.to_string_lossy().into(),output_dir:dir.to_string_lossy().into(),operation:MediaOperation::SubtitleConvert,output_format:OutputFormat::Vtt,..Default::default()};let mut c=command::build(&request,info,&target)?;if let Some(i)=index{if let Some(n)=c.args.iter().position(|a|a=="-map"){c.args[n+1]=format!("0:{i}");}}self.media.auxiliary(session,&c.args,30)?;self.media.authorize_asset(session,&target)?;Ok(target.to_string_lossy().into())}
 pub fn subtitle(&self,path:&Path,session:&str)->AppResult<PlayerSubtitle>{crate::media::trim::timeline::validate_session(session)?;let p=regular_file(&path.to_string_lossy())?;crate::media::subtitle::extension(&p.to_string_lossy())?;if fs::metadata(&p).map_err(|_|"无法读取字幕")?.len()>8*1024*1024{return Err("字幕超过8MiB，请选择较小文件".into());}let info=self.media.probe(&p.to_string_lossy())?;let converted=self.convert_subtitle(&p,&info,session,None)?;Ok(PlayerSubtitle{label:p.file_name().unwrap_or_default().to_string_lossy().into(),path:converted})}
}
