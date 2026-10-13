use serde::{Deserialize,Serialize};
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct LearningCategory{pub id:String,pub name:String,pub description:Option<String>,pub icon:Option<String>,pub color:Option<String>,pub sort_order:i64}
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct LearningSeries{pub id:String,pub category_id:Option<String>,pub title:String,pub description:Option<String>,pub cover_path:Option<String>,pub source_type:Option<String>,pub source_url:Option<String>,pub source_playlist_id:Option<String>,pub author:Option<String>,pub total_items:i64,pub completed_items:i64,pub total_duration_ms:i64,pub favorite:bool,pub sort_order:i64,pub created_at:i64,pub updated_at:i64}
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct PlaybackProgress{pub item_id:String,pub position_ms:i64,pub duration_ms:i64,pub completed:bool,pub playback_rate:f64,pub last_played_at:Option<i64>}
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct LearningItem{pub id:String,pub series_id:String,pub title:String,pub file_path:Option<String>,pub source_url:Option<String>,pub source_video_id:Option<String>,pub source_playlist_index:Option<i64>,pub duration_ms:Option<i64>,pub sort_order:i64,pub thumbnail_path:Option<String>,pub download_task_id:Option<String>,pub available:bool,pub download_state:String,pub progress:PlaybackProgress}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct VideoNote{pub id:String,pub item_id:String,pub time_ms:i64,pub content:String,pub created_at:i64,pub updated_at:i64}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct LearningSettings{pub autoplay_next:bool,pub completion_threshold:f64,pub default_playback_rate:f64,pub restore_position:bool,pub auto_subtitles:bool}
impl Default for LearningSettings{fn default()->Self{Self{autoplay_next:true,completion_threshold:0.9,default_playback_rate:1.,restore_position:true,auto_subtitles:true}}}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub struct SeriesDetail{pub series:LearningSeries,pub items:Vec<LearningItem>,pub page:u32,pub total:i64}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub struct ContinueLearningItem{pub series:LearningSeries,pub item:LearningItem}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub struct LibraryHome{pub categories:Vec<LearningCategory>,pub series:Vec<LearningSeries>,pub total:i64,pub page:u32,pub continue_learning:Vec<ContinueLearningItem>,pub settings:LearningSettings}
#[derive(Debug,Clone,Deserialize,Default)]
#[serde(rename_all="camelCase",default,deny_unknown_fields)]
pub struct CategoryInput{pub id:Option<String>,pub name:String,pub description:Option<String>,pub icon:Option<String>,pub color:Option<String>}
#[derive(Debug,Clone,Deserialize,Default)]
#[serde(rename_all="camelCase",default,deny_unknown_fields)]
pub struct SeriesInput{pub id:Option<String>,pub category_id:Option<String>,pub title:String,pub description:Option<String>,pub cover_path:Option<String>,pub source_url:Option<String>,pub favorite:bool}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct ProgressInput{pub item_id:String,pub position_ms:i64,pub duration_ms:i64,pub playback_rate:f64,pub ended:bool,pub sequence:u64}
#[derive(Debug,Deserialize)]
#[serde(tag="type",rename_all="camelCase",rename_all_fields="camelCase",deny_unknown_fields)]
pub enum LearningChange{SaveCategory{category:CategoryInput},DeleteCategory{id:String,move_to_uncategorized:bool},SaveSeries{series:SeriesInput},DeleteSeries{id:String,delete_files:bool,confirmed:bool},RenameItem{id:String,title:String},DeleteItem{id:String,delete_files:bool,confirmed:bool},Move{scope:String,id:String,before_id:Option<String>,direction:Option<String>},MarkCompleted{id:String,completed:bool},DeleteNote{id:String},SaveSettings{settings:LearningSettings}}
