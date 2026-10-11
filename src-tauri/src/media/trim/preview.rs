use crate::{domain::AppResult,ffmpeg::{models::*,output,runner::MediaService}};
use tauri::Manager;
impl MediaService {
 pub fn editor_preview(&self,path:&str,session:&str,proxy:bool)->AppResult<String>{
  self.ensure_ready()?;super::timeline::validate_session(session)?;
  let input=output::input(path)?;
  let target=if proxy {
   let info=self.probe_file(&input,&format!("{session}-probe"),false)?;
   let duration=super::processor::validate_duration(info.duration)?;
   if duration>21600. {return Err("超过6小时的视频请使用原始预览或外部播放器".into());}
   let dir=self.preview_directory(session)?;
   let target=dir.join(format!("preview-{}.mp4",crate::ffmpeg::runner::new_id()));
   let request=MediaRequest{input_path:input.to_string_lossy().into(),output_dir:dir.to_string_lossy().into(),height:Some(720),quality:Quality::Small,..Default::default()};
   let mut command=crate::ffmpeg::command::build(&request,&info,&target)?;
   if let Some(n)=command.args.iter().position(|a|a=="medium"){command.args[n]="ultrafast".into();}
   self.auxiliary(session,&command.args,300)?;
   let generated=self.probe_file(&target,&format!("{session}-verify"),false)?;
   if generated.duration.is_none_or(|d|d+1.<duration){return Err("预览代理不完整，请重试".into());}
   target
  }else{input};
  self.authorize_asset(session,&target)?;
  Ok(target.to_string_lossy().into_owned())
 }
 pub(crate) fn authorize_asset(&self,session:&str,path:&std::path::Path)->AppResult<()> {
  let state=self.state.lock().unwrap();
  if state.exiting||state.cancelled_previews.contains(session){return Err("预览已取消".into());}
  self.app.asset_protocol_scope().allow_file(path).map_err(|_|"无法授权预览文件".to_string())
 }
}
