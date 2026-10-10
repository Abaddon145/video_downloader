pub mod batch;
pub mod domain;
pub mod engine;
pub mod ffmpeg;
pub mod models;
pub mod naming;
pub mod native;
pub mod service;

use domain::AppResult;
use models::*;
use service::Service;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};

type AppService<'a> = State<'a, Arc<Service>>;

#[tauri::command]
fn get_snapshot(service: AppService<'_>) -> AppSnapshot {
    service.snapshot()
}

#[tauri::command]
async fn preview_sources(
    service: AppService<'_>,
    urls: Vec<String>,
) -> AppResult<Vec<PreviewResult>> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.preview(urls))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn cancel_preview(service: AppService<'_>) {
    service.cancel_preview();
}
#[tauri::command]
fn enqueue_downloads(
    service: AppService<'_>,
    requests: Vec<DownloadRequest>,
) -> AppResult<Vec<DownloadTask>> {
    service.enqueue(requests)
}
#[tauri::command]
fn control_task(service: AppService<'_>, id: String, action: String) -> AppResult<()> {
    service.control(&id, &action)
}
#[tauri::command]
fn remove_task(service: AppService<'_>, id: String) -> AppResult<()> {
    service.remove_task(&id)
}
#[tauri::command]
async fn batch_task_action(
    service: AppService<'_>,
    ids: Vec<String>,
    action: batch::BatchAction,
) -> AppResult<batch::BatchResult> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.batch_task_action(ids, action))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn save_settings(service: AppService<'_>, input: SettingsInput) -> AppResult<AppSettings> {
    service.save_settings(input)
}
#[tauri::command]
fn import_cookies(service: AppService<'_>, path: String) -> AppResult<()> {
    service.import_cookies(&path)
}
#[tauri::command]
fn save_download_presets(
    service: AppService<'_>,
    presets: Vec<DownloadPreset>,
) -> AppResult<AppSettings> {
    service.save_presets(presets)
}
#[tauri::command]
fn open_task_target(service: AppService<'_>, id: String, folder: bool) -> AppResult<()> {
    service::open_target(&service, &id, folder)
}
#[tauri::command]
fn open_source(url: String) -> AppResult<()> {
    service::shell_open(&domain::validate_url(&url)?)
}
#[tauri::command]
async fn check_engine_update(service: AppService<'_>) -> AppResult<EngineUpdate> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.check_update())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn update_engine(service: AppService<'_>) -> AppResult<EngineInfo> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.update_engine())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn exit_app(app: tauri::AppHandle, service: AppService<'_>) {
    service.shutdown();
    app.exit(0);
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn run() {
    let _instance = match native::single_instance() {
        Ok(Some(handle)) => handle,
        Ok(None) => return,
        Err(error) => {
            native::message(&error);
            return;
        }
    };
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            save_download_presets,
            batch_task_action,
            preview_sources,
            cancel_preview,
            enqueue_downloads,
            control_task,
            remove_task,
            save_settings,
            import_cookies,
            open_task_target,
            open_source,
            check_engine_update,
            update_engine,
            exit_app
        ])
        .setup(|app| {
            let service = Service::new(app.handle().clone()).map_err(std::io::Error::other)?;
            app.manage(service.clone());
            let show = MenuItem::with_id(app, "show", "打开视频下载器", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "保存任务并退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &exit])?;
            TrayIconBuilder::with_id("downloads")
                .icon(app.default_window_icon().ok_or("缺少应用图标")?.clone())
                .tooltip("视频下载器 · 关闭窗口后继续下载")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "exit" => {
                        app.state::<Arc<Service>>().shutdown();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        }
                    ) {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            service.start();
            show_main(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!());
    match application {
        Ok(app) => app.run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                app.state::<Arc<Service>>().shutdown();
            }
        }),
        Err(error) => native::message(&format!("软件启动失败：{error}")),
    }
}
