use crate::domain::AppResult;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Write},
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
    #[serde(default)]
    pub sidebar_collapsed: bool,
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
    #[serde(default)]
    pub active_preset: Option<String>,
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
                sidebar_collapsed: false,
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
            active_preset: Some("builtin.default".into()),
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
                || p.id.starts_with("builtin.")
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
        if self.active_preset.as_ref().is_some_and(|id| {
            id.len() > 100
                || id.chars().any(char::is_control)
                || ![
                    "builtin.default",
                    "builtin.learning",
                    "builtin.focus",
                    "builtin.batch",
                    "builtin.knowledge",
                    "builtin.theater",
                ]
                .contains(&id.as_str())
                    && !ids.contains(id)
        }) {
            return Err("当前预设 ID 无效".into());
        }
        Ok(())
    }
}
fn settings_bytes(settings: &WorkspaceSettings) -> AppResult<Vec<u8>> {
    settings.validate()?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|_| "无法序列化工作区配置")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("工作区配置超过128KiB".into());
    }
    Ok(bytes)
}
fn json_path(path: &Path) -> AppResult<()> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err("请选择本地 .json 工作区配置文件".into());
    }
    Ok(())
}
fn read_import(path: &Path) -> AppResult<WorkspaceSettings> {
    json_path(path)?;
    let file = std::fs::File::open(path).map_err(|_| "无法读取工作区配置文件")?;
    let metadata = file.metadata().map_err(|_| "无法读取工作区配置文件信息")?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err("工作区配置必须是128KiB以内的普通文件".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "读取工作区配置文件失败")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("工作区配置超过128KiB".into());
    }
    let settings: WorkspaceSettings = serde_json::from_slice(&bytes)
        .map_err(|_| "工作区配置JSON无效或包含未知字段；原文件已保留")?;
    settings.validate()?;
    Ok(settings)
}
fn write_export(path: &Path, bytes: &[u8]) -> AppResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
    json_path(path)?;
    let temporary = path.with_extension(format!("{}.new", crate::ffmpeg::runner::new_id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "无法创建工作区导出文件，请检查目录权限")?;
    let written = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    let result = written
        .map_err(|_| "工作区配置写入未完成，请检查目标磁盘".to_owned())
        .and_then(|_| {
            let wide = |p: &Path| {
                p.as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
            };
            // Publish only a complete new file; never replace another user file, even after a dialog race.
            if unsafe {
                MoveFileExW(
                    wide(&temporary).as_ptr(),
                    wide(path).as_ptr(),
                    MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                Err("无法保存工作区配置；目标可能已存在，请使用新的文件名".into())
            } else {
                Ok(())
            }
        });
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
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
        let bytes = settings_bytes(&settings)?;
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
#[tauri::command]
pub async fn workspace_import(app: tauri::AppHandle) -> AppResult<Option<WorkspaceSettings>> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .add_filter("工作区 JSON", &["json"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = path.into_path().map_err(|_| "请选择本地JSON文件")?;
        read_import(&path).map(Some)
    })
    .await
    .map_err(|_| "工作区配置导入未完成")?
}
#[tauri::command]
pub async fn workspace_export(
    app: tauri::AppHandle,
    settings: WorkspaceSettings,
) -> AppResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = settings_bytes(&settings)?;
        let Some(path) = app
            .dialog()
            .file()
            .add_filter("工作区 JSON", &["json"])
            .set_file_name("映流工作区.json")
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = path.into_path().map_err(|_| "请选择本地JSON文件路径")?;
        write_export(&path, &bytes)?;
        Ok(true)
    })
    .await
    .map_err(|_| "工作区配置导出未完成")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_workspace_and_active_preset_round_trip_without_losing_layout() {
        let mut legacy = serde_json::to_value(WorkspaceSettings::default()).unwrap();
        legacy["currentLayout"]["library"]["view"] = "table".into();
        legacy.as_object_mut().unwrap().remove("activePreset");
        legacy["appearance"]
            .as_object_mut()
            .unwrap()
            .remove("sidebarCollapsed");
        let restored: WorkspaceSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.current_layout.library.view, "table");
        assert_eq!(restored.active_preset, None);
        assert!(!restored.appearance.sidebar_collapsed);
        let mut applied = restored;
        applied.active_preset = Some("builtin.theater".into());
        assert_eq!(
            serde_json::from_slice::<WorkspaceSettings>(&settings_bytes(&applied).unwrap())
                .unwrap()
                .active_preset
                .as_deref(),
            Some("builtin.theater")
        );
    }
    #[test]
    fn workspace_import_rejects_unknown_active_preset_and_credentials() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("invalid.json");
        let mut value = serde_json::to_value(WorkspaceSettings::default()).unwrap();
        for id in [
            "builtin.missing",
            "custom-missing",
            "bad\0id",
            "long-id-that-does-not-exist",
        ] {
            value["activePreset"] = id.into();
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(read_import(&path).is_err());
        }
        value["activePreset"] = serde_json::Value::Null;
        value["apiKey"] = "private".into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_import(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn workspace_presets_reject_builtin_identity_and_control_characters() {
        let mut settings = WorkspaceSettings::default();
        settings.presets.push(WorkspacePreset {
            id: "builtin.video".into(),
            name: "自定义".into(),
            layout: settings.current_layout.clone(),
        });
        assert!(settings.validate().is_err());
        for invalid in ["bad\0id", "bad\u{7f}id", "bad\u{85}id"] {
            settings.presets[0].id = invalid.into();
            assert!(settings.validate().is_err());
            settings.presets[0].id = "custom.test".into();
            settings.presets[0].name = invalid.into();
            assert!(settings.validate().is_err());
            settings.presets[0].name = "自定义".into();
        }
        assert!(settings.validate().is_ok());
    }
    #[test]
    fn workspace_import_is_strict_bounded_and_does_not_save_settings() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("导入 工作区.json");
        let mut value = serde_json::to_value(WorkspaceSettings::default()).unwrap();
        value["appearance"]["theme"] = "light".into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(read_import(&path).unwrap().appearance.theme, "light");
        assert!(!dir.join("workspace.json").exists());
        for key in ["apiKey", "cookie", "appSettings", "unknown"] {
            value[key] = "private".into();
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(read_import(&path).is_err());
            value.as_object_mut().unwrap().remove(key);
        }
        value["appearance"]["cookie"] = "private".into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_import(&path).is_err());
        value["appearance"]
            .as_object_mut()
            .unwrap()
            .remove("cookie");
        let mut oversized = vec![b' '; MAX_BYTES as usize];
        oversized.extend(serde_json::to_vec(&value).unwrap());
        assert!(serde_json::from_slice::<WorkspaceSettings>(&oversized).is_ok());
        std::fs::write(&path, &oversized).unwrap();
        assert!(read_import(&path).unwrap_err().contains("128KiB"));
        for invalid in [
            b"{broken".to_vec(),
            {
                value["version"] = 2.into();
                serde_json::to_vec(&value).unwrap()
            },
            {
                value["version"] = 1.into();
                value["appearance"]["fontSize"] = 999.into();
                serde_json::to_vec(&value).unwrap()
            },
        ] {
            std::fs::write(&path, &invalid).unwrap();
            assert!(read_import(&path).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), invalid);
        }
        assert!(!dir.join("workspace.json").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn workspace_export_publishes_complete_json_without_overwriting_existing_files() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("导出 工作区.json");
        let mut settings = WorkspaceSettings::default();
        settings.appearance.density = "compact".into();
        let bytes = settings_bytes(&settings).unwrap();
        write_export(&path, &bytes).unwrap();
        assert_eq!(read_import(&path).unwrap().appearance.density, "compact");
        assert!(write_export(&path, b"replace").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(write_export(&dir.join("wrong.txt"), &bytes).is_err());
        settings.appearance.sidebar_width = 0.;
        assert!(settings_bytes(&settings).is_err());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn workspace_round_trip_rejects_invalid_and_future_data_without_overwriting() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir_all(&dir).unwrap();
        let store = WorkspaceStore::new(&dir);
        let mut settings = WorkspaceSettings::default();
        settings.current_layout.library.view = "table".into();
        settings.current_layout.library.category_width = 280.;
        settings.active_preset = Some("builtin.batch".into());
        store.save(settings.clone()).unwrap();
        assert_eq!(store.get().unwrap().current_layout.library.view, "table");
        assert_eq!(
            store.get().unwrap().active_preset.as_deref(),
            Some("builtin.batch")
        );
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
