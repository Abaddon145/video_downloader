use crate::domain::AppResult;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
};

const MAX_BYTES: u64 = 131_072;
const COLUMNS: &[&str] = &[
    "title",
    "category",
    "items",
    "duration",
    "progress",
    "aiStatus",
    "availability",
    "lastPlayed",
    "updated",
    "actions",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAppearance {
    pub theme: String,
    pub accent: String,
    pub font_size: f64,
    pub density: String,
    pub sidebar_width: f64,
    pub motion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryLayout {
    pub view: String,
    pub category_width: f64,
    pub category_collapsed: bool,
    pub card_size: f64,
    pub cards_per_row: Option<u8>,
    pub cover_ratio: String,
    pub show_progress: bool,
    pub show_ai_status: bool,
    pub show_continue: bool,
    pub sort: String,
    pub columns: Vec<String>,
    pub column_widths: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerLayout {
    pub layout: String,
    pub panel_size: f64,
    pub panel_collapsed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeLayout {
    pub tree_width: f64,
    pub info_width: f64,
    pub tree_collapsed: bool,
    pub info_collapsed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceLayout {
    pub library: LibraryLayout,
    pub player: PlayerLayout,
    pub knowledge: KnowledgeLayout,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePreset {
    pub id: String,
    pub name: String,
    pub layout: WorkspaceLayout,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSettings {
    pub version: u32,
    pub appearance: WorkspaceAppearance,
    pub current_layout: WorkspaceLayout,
    pub presets: Vec<WorkspacePreset>,
}
impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            version: 1,
            appearance: WorkspaceAppearance {
                theme: "dark".into(),
                accent: "#38BDF8".into(),
                font_size: 14.,
                density: "standard".into(),
                sidebar_width: 194.,
                motion: "system".into(),
            },
            current_layout: WorkspaceLayout {
                library: LibraryLayout {
                    view: "cards".into(),
                    category_width: 224.,
                    category_collapsed: false,
                    card_size: 260.,
                    cards_per_row: None,
                    cover_ratio: "16:9".into(),
                    show_progress: true,
                    show_ai_status: true,
                    show_continue: true,
                    sort: "manual".into(),
                    columns: COLUMNS.iter().map(|s| s.to_string()).collect(),
                    column_widths: BTreeMap::new(),
                },
                player: PlayerLayout {
                    layout: "standard".into(),
                    panel_size: 360.,
                    panel_collapsed: false,
                },
                knowledge: KnowledgeLayout {
                    tree_width: 240.,
                    info_width: 280.,
                    tree_collapsed: false,
                    info_collapsed: false,
                },
            },
            presets: Vec::new(),
        }
    }
}
fn range(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}
impl WorkspaceLayout {
    fn valid(&self) -> bool {
        let b = &self.library;
        let p = &self.player;
        let k = &self.knowledge;
        ["cards", "list", "table"].contains(&b.view.as_str())
            && range(b.category_width, 160., 480.)
            && range(b.card_size, 160., 480.)
            && b.cards_per_row.is_none_or(|n| (1..=8).contains(&n))
            && ["16:9", "4:3", "square"].contains(&b.cover_ratio.as_str())
            && [
                "manual",
                "title",
                "updated",
                "lastPlayed",
                "progress",
                "duration",
            ]
            .contains(&b.sort.as_str())
            && b.columns.contains(&"title".into())
            && b.columns.contains(&"actions".into())
            && b.columns.len() <= COLUMNS.len()
            && b.columns.iter().all(|c| COLUMNS.contains(&c.as_str()))
            && b.columns.iter().collect::<HashSet<_>>().len() == b.columns.len()
            && b.column_widths
                .iter()
                .all(|(c, w)| COLUMNS.contains(&c.as_str()) && range(*w, 64., 800.))
            && ["standard", "stacked", "theater", "focus"].contains(&p.layout.as_str())
            && range(p.panel_size, 220., 720.)
            && range(k.tree_width, 160., 480.)
            && range(k.info_width, 200., 600.)
    }
}
impl WorkspaceSettings {
    pub fn validate(&self) -> AppResult<()> {
        let a = &self.appearance;
        if self.version != 1
            || !["dark", "light", "system"].contains(&a.theme.as_str())
            || a.accent.len() != 7
            || !a.accent.starts_with('#')
            || !a.accent.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            || !range(a.font_size, 12., 20.)
            || !["comfortable", "standard", "compact"].contains(&a.density.as_str())
            || !range(a.sidebar_width, 160., 320.)
            || !["system", "reduced", "full"].contains(&a.motion.as_str())
            || !self.current_layout.valid()
            || self.presets.len() > 50
        {
            return Err("工作区配置无效或版本不受支持".into());
        }
        let mut ids = HashSet::new();
        for p in &self.presets {
            if p.id.is_empty()
                || p.id.len() > 100
                || p.name.trim().is_empty()
                || p.name.chars().count() > 80
                || p.id.chars().chain(p.name.chars()).any(char::is_control)
                || !ids.insert(&p.id)
                || !p.layout.valid()
            {
                return Err("工作区预设无效".into());
            }
        }
        Ok(())
    }
}
pub struct WorkspaceStore {
    path: PathBuf,
    guard: Mutex<()>,
}
impl WorkspaceStore {
    pub fn new(data: &Path) -> Self {
        Self {
            path: data.join("workspace.json"),
            guard: Mutex::new(()),
        }
    }
    pub fn get(&self) -> AppResult<WorkspaceSettings> {
        let _guard = self.guard.lock().map_err(|_| "工作区设置暂不可用")?;
        // Keep unknown/corrupt settings untouched so a newer app or the user can recover them.
        Ok(std::fs::metadata(&self.path)
            .ok()
            .filter(|m| m.len() <= MAX_BYTES)
            .and_then(|_| std::fs::read(&self.path).ok())
            .and_then(|bytes| serde_json::from_slice::<WorkspaceSettings>(&bytes).ok())
            .filter(|s| s.validate().is_ok())
            .unwrap_or_default())
    }
    pub fn save(&self, settings: WorkspaceSettings) -> AppResult<()> {
        settings.validate()?;
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("工作区配置超过128KB".into());
        }
        let _guard = self.guard.lock().map_err(|_| "工作区设置暂不可用")?;
        crate::native::save_atomic(&self.path, &bytes)
    }
}
#[tauri::command]
pub fn workspace_get(workspace: tauri::State<'_, WorkspaceStore>) -> AppResult<WorkspaceSettings> {
    workspace.get()
}
#[tauri::command]
pub fn workspace_save(
    workspace: tauri::State<'_, WorkspaceStore>,
    settings: WorkspaceSettings,
) -> AppResult<()> {
    workspace.save(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_round_trip_rejects_invalid_and_future_data_without_overwriting() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir_all(&dir).unwrap();
        let store = WorkspaceStore::new(&dir);
        let mut settings = WorkspaceSettings::default();
        settings.current_layout.library.view = "table".into();
        settings.current_layout.library.category_width = 280.;
        store.save(settings.clone()).unwrap();
        assert_eq!(store.get().unwrap().current_layout.library.view, "table");
        settings.appearance.font_size = 999.;
        assert!(store.save(settings).is_err());
        let mut settings = WorkspaceSettings::default();
        settings
            .current_layout
            .library
            .columns
            .retain(|c| c != "title");
        assert!(store.save(settings.clone()).is_err());
        settings.current_layout.library.columns.push("title".into());
        settings
            .current_layout
            .library
            .columns
            .retain(|c| c != "actions");
        assert!(store.save(settings).is_err());
        assert_eq!(
            store.get().unwrap().current_layout.library.category_width,
            280.
        );
        for bytes in [b"{broken".as_slice(), b"{\"version\":999}".as_slice()] {
            std::fs::write(dir.join("workspace.json"), bytes).unwrap();
            assert_eq!(store.get().unwrap().current_layout.library.view, "cards");
            assert_eq!(std::fs::read(dir.join("workspace.json")).unwrap(), bytes);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
