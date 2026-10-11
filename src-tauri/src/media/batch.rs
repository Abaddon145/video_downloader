use crate::domain::AppResult;
pub fn discover(paths:&[String])->AppResult<Vec<String>>{
 use std::{collections::HashSet,fs,path::PathBuf,os::windows::fs::MetadataExt};
 if paths.is_empty()||paths.len()>500{return Err("请选择1至500个文件或目录".into());}
 let mut pending=Vec::<PathBuf>::new();for path in paths{crate::ffmpeg::command::valid_path(path)?;pending.push(path.into());}
 let mut seen=HashSet::new();let mut files=Vec::new();let mut visited=0;
 while let Some(path)=pending.pop(){visited+=1;if visited>10000{return Err("目录项目过多，请选择更小的文件夹".into());}
  let m=fs::symlink_metadata(&path).map_err(|_|format!("无法读取：{}",path.to_string_lossy()))?;
  if m.file_attributes()&0x400!=0{continue;} // Never follow Windows reparse points (including junctions).
  let p=fs::canonicalize(&path).map_err(|_|"无法读取媒体路径")?;crate::ffmpeg::command::valid_path(&p.to_string_lossy())?;
  if !seen.insert(p.to_string_lossy().to_lowercase()){continue;}
  if m.is_dir(){let mut children=fs::read_dir(p).map_err(|_|"文件夹没有读取权限")?.map(|e|e.map(|e|e.path()).map_err(|_|"文件夹项目无法读取")).collect::<Result<Vec<_>,_>>()?;children.sort();pending.extend(children.into_iter().rev());}
  else if m.is_file()&&p.extension().is_some_and(|e|["mp4","mkv","mov","webm","avi","m4v","mp3","aac","flac","wav","m4a","opus","ogg","wma","wmv","ts","flv"].contains(&e.to_string_lossy().to_ascii_lowercase().as_str())){files.push(p.to_string_lossy().into_owned());if files.len()>500{return Err("单批最多500个媒体文件".into());}}
 }files.sort();Ok(files)
}
#[derive(serde::Serialize)]#[serde(rename_all="camelCase")]
pub struct BatchItem {pub path:String,pub task_id:Option<String>,pub error:Option<String>}
#[derive(serde::Serialize)]#[serde(rename_all="camelCase")]
pub struct BatchMediaTask {pub id:String,pub items:Vec<BatchItem>}
impl crate::ffmpeg::runner::MediaService {
 pub fn create_batch(&self,request:crate::ffmpeg::models::MediaRequest,paths:Vec<String>)->AppResult<BatchMediaTask>{
  if !matches!(request.operation,crate::ffmpeg::models::MediaOperation::Transcode|crate::ffmpeg::models::MediaOperation::Compress|crate::ffmpeg::models::MediaOperation::ExtractAudio|crate::ffmpeg::models::MediaOperation::Screenshot){return Err("批量支持转换、压缩、提取音频和截图".into());}
  let files=discover(&paths)?;if files.is_empty(){return Err("没有找到受支持的媒体文件".into());}let id=crate::ffmpeg::runner::new_id();let mut items=Vec::new();
  for path in files {let mut r=request.clone();r.input_path=path.clone();r.batch_id=Some(id.clone());r.source_download_id=None;let (task_id,error)=match self.create(r){Ok(t)=>(Some(t.id),None),Err(e)=>(None,Some(e))};items.push(BatchItem{path,task_id,error});}
  Ok(BatchMediaTask{id,items})
 }
}

#[cfg(test)]mod tests{use super::*;#[test]fn batch_folders_collect_supported_files_once_and_reject_remote(){let dir=std::env::temp_dir().join(crate::ffmpeg::runner::new_id());std::fs::create_dir(&dir).unwrap();std::fs::write(dir.join("中文.mp4"),b"fixture").unwrap();std::fs::write(dir.join("b.mkv"),b"fixture").unwrap();std::fs::write(dir.join("ignore.txt"),b"fixture").unwrap();let p=dir.to_string_lossy().into_owned();assert_eq!(discover(&[p.clone(),p]).unwrap().len(),2);assert!(discover(&["https://x/file".into()]).is_err());std::fs::remove_dir_all(dir).unwrap();}}
