use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeSettings {
    pub vault_path: Option<String>,
    pub vault_id: Option<String>,
    #[serde(default = "root_folder")]
    pub root_folder: String,
    #[serde(default = "attachments_folder")]
    pub attachments_folder: String,
    #[serde(default)]
    pub obsidian: Option<ObsidianBinding>,
}
fn root_folder() -> String {
    "映流".into()
}
fn attachments_folder() -> String {
    "Attachments".into()
}
impl KnowledgeSettings {
    pub fn empty() -> Self {
        Self {
            vault_path: None,
            vault_id: None,
            root_folder: root_folder(),
            attachments_folder: attachments_folder(),
            obsidian: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObsidianKind {
    VaultId,
    VaultName,
    AbsolutePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObsidianBinding {
    pub yingliu_vault_id: String,
    pub kind: ObsidianKind,
    pub obsidian_vault_identifier: Option<String>,
    pub registration_confirmed: bool,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ObsidianConnectionState {
    Unconfigured,
    ProtocolUnavailable,
    RegistrationPending,
    VerificationPending,
    UserConfirmed,
    LaunchFailed,
    StaleVault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ObsidianLaunchStatus {
    LaunchAccepted,
    LaunchFailed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsidianOpenResult {
    pub vault_id: String,
    pub path: String,
    pub status: ObsidianLaunchStatus,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsidianConnection {
    pub vault_id: String,
    pub vault_path: String,
    pub binding: Option<ObsidianBinding>,
    pub state: ObsidianConnectionState,
    pub protocol_available: bool,
    pub last_request: Option<ObsidianOpenResult>,
}

impl Default for KnowledgeSettings {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeView<T> {
    pub vault_id: String,
    pub data: T,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeWrite {
    pub vault_id: String,
    pub outcome: super::vault::WriteOutcome,
    pub index_warning: Option<String>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMutation {
    pub vault_id: String,
    pub path: String,
    pub index_warning: Option<String>,
}
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub indexed: usize,
    pub skipped: usize,
    pub complete: bool,
    pub warnings: Vec<String>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAsset {
    pub mime: String,
    pub base64: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetadata {
    pub path: String,
    pub favorite: bool,
    pub template: bool,
    pub available: bool,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetadataPage {
    pub items: Vec<NoteMetadata>,
    pub total: u64,
    pub page: u32,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersion {
    pub id: String,
    pub path: String,
    pub hash: String,
    pub bytes: u64,
    pub created_at: i64,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersionPage {
    pub items: Vec<NoteVersion>,
    pub total: u64,
    pub page: u32,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersionContent {
    #[serde(flatten)]
    pub version: NoteVersion,
    pub text: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteAttachment {
    pub path: String,
    pub name: String,
    pub hash: String,
    pub bytes: u64,
    pub imported_at: i64,
    pub state: String,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteAttachmentPage {
    pub items: Vec<NoteAttachment>,
    pub total: u64,
    pub page: u32,
}
