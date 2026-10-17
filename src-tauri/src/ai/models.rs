use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    Quick,
    Standard,
    Deep,
}
impl AnalysisMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoAnalysis {
    pub title: Option<String>,
    pub options: AnalysisOptions,
    pub transcript_done: bool,
    pub keyframes_done: bool,
    pub id: String,
    pub item_id: Option<String>,
    pub source_path: String,
    pub source_fingerprint: String,
    pub status: String,
    pub analysis_mode: AnalysisMode,
    pub model: String,
    pub prompt_version: String,
    pub language: Option<String>,
    pub duration_ms: Option<i64>,
    pub progress: f64,
    pub stage: Option<String>,
    pub summary: Option<VideoAnalysisResult>,
    pub markdown_path: Option<String>,
    pub markdown_vault_id: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoAnalysisResult {
    pub content_type: String,
    pub one_sentence_summary: String,
    pub overview: String,
    pub chapters: Vec<Chapter>,
    pub key_points: Vec<KeyPoint>,
    pub tutorial_steps: Vec<TutorialStep>,
    pub terms: Vec<Term>,
    pub action_items: Vec<String>,
    pub tags: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Chapter {
    pub start_ms: i64,
    pub end_ms: i64,
    pub title: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub keyframe_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyPoint {
    pub time_ms: i64,
    pub title: String,
    pub content: String,
    pub importance: u8,
    pub source_segment_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TutorialStep {
    pub order: u32,
    pub time_ms: i64,
    pub title: String,
    pub description: String,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Term {
    pub name: String,
    pub definition: String,
    pub time_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub analysis_id: String,
    pub segment_index: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisPage {
    pub items: Vec<VideoAnalysis>,
    pub total: u64,
    pub page: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPage {
    pub items: Vec<TranscriptSegment>,
    pub total: u64,
    pub page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisKeyframe {
    pub id: String,
    pub analysis_id: String,
    pub time_ms: i64,
    pub path: String,
    pub sha256: String,
    pub scene_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AnalysisOptions {
    pub local_only: bool,
    pub transcribe_audio: bool,
    pub allow_transcript: bool,
    pub allow_images: bool,
    pub whisper_model: String,
    pub auto_generate_note: bool,
}
impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            local_only: true,
            transcribe_audio: true,
            allow_transcript: true,
            allow_images: true,
            whisper_model: "base".into(),
            auto_generate_note: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisRequest {
    #[serde(default)]
    pub force: bool,
    pub path: String,
    pub item_id: Option<String>,
    pub mode: AnalysisMode,
    pub transcribe_audio: bool,
    pub allow_transcript: bool,
    pub allow_images: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryVideo {
    pub item_id: String,
    pub title: String,
    pub series_title: String,
    pub path: Option<String>,
    pub duration_ms: Option<i64>,
    pub available: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryVideoPage {
    pub items: Vec<LibraryVideo>,
    pub total: u64,
    pub page: u32,
}
