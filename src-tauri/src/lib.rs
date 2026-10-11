pub mod batch;
pub mod domain;
pub mod engine;
pub mod ffmpeg;
pub mod media;
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
fn exit_app(app: tauri::AppHandle, service: AppService<'_>, media: AppMedia<'_>) -> AppResult<()> {
    media.shutdown()?;
    service.shutdown();
    app.exit(0);
    Ok(())
}

type AppMedia<'a> = State<'a, Arc<ffmpeg::runner::MediaService>>;
#[tauri::command]
fn get_media_snapshot(media: AppMedia<'_>) -> ffmpeg::models::MediaSnapshot {
    media.snapshot()
}
#[tauri::command]
async fn probe_media(media: AppMedia<'_>, path: String) -> AppResult<ffmpeg::models::MediaInfo> {
    let media = media.inner().clone();
    tauri::async_runtime::spawn_blocking(move || media.probe(&path))
        .await
        .map_err(|_| "媒体探测线程异常")?
}
#[tauri::command]
fn create_media_task(
    media: AppMedia<'_>,
    request: ffmpeg::models::MediaRequest,
) -> AppResult<ffmpeg::models::MediaTask> {
    media.create(request)
}
#[tauri::command]
fn cancel_media_task(media: AppMedia<'_>, id: String) -> AppResult<()> {
    media.cancel(&id)
}
#[tauri::command]
fn retry_media_task(media: AppMedia<'_>, id: String) -> AppResult<()> {
    media.retry(&id)
}
#[tauri::command]
fn open_media_output(media: AppMedia<'_>, id: String, folder: bool) -> AppResult<()> {
    media.open_output(&id, folder)
}

#[tauri::command]
async fn discover_media(paths: Vec<String>) -> AppResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || media::batch::discover(&paths))
        .await
        .map_err(|_| "批量扫描线程异常")?
}
#[tauri::command]
async fn create_batch_media(
    media: AppMedia<'_>,
    request: ffmpeg::models::MediaRequest,
    paths: Vec<String>,
) -> AppResult<media::batch::BatchMediaTask> {
    let media = media.inner().clone();
    tauri::async_runtime::spawn_blocking(move || media.create_batch(request, paths))
        .await
        .map_err(|_| "批量入队线程异常")?
}
#[tauri::command]
async fn get_media_capabilities(media: AppMedia<'_>) -> AppResult<Vec<String>> {
    let media = media.inner().clone();
    tauri::async_runtime::spawn_blocking(move || media.capabilities("gpu-settings"))
        .await
        .map_err(|_| "GPU 检测线程异常")?
}
#[tauri::command]
fn get_media_settings(media: AppMedia<'_>) -> media::preset::MediaProSettings {
    media.pro_settings()
}
#[tauri::command]
fn save_media_settings(
    media: AppMedia<'_>,
    settings: media::preset::MediaProSettings,
) -> AppResult<media::preset::MediaProSettings> {
    media.save_pro_settings(settings)
}
#[tauri::command]
async fn preview_media(
    media: AppMedia<'_>,
    path: String,
    session: String,
    proxy: bool,
) -> AppResult<String> {
    let media = media.inner().clone();
    tauri::async_runtime::spawn_blocking(move || media.editor_preview(&path, &session, proxy))
        .await
        .map_err(|_| "预览线程异常")?
}
#[tauri::command]
async fn media_thumbnails(
    media: AppMedia<'_>,
    path: String,
    session: String,
    offset: u32,
) -> AppResult<Vec<media::trim::thumbnail::TimelineThumbnail>> {
    let media = media.inner().clone();
    tauri::async_runtime::spawn_blocking(move || media.editor_thumbnails(&path, &session, offset))
        .await
        .map_err(|_| "缩略图线程异常")?
}
#[tauri::command]
fn cancel_media_preview(media: AppMedia<'_>, session: String) -> AppResult<()> {
    media.cancel_editor(&session)
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
            discover_media,
            create_batch_media,
            get_media_capabilities,
            get_media_settings,
            save_media_settings,
            preview_media,
            media_thumbnails,
            cancel_media_preview,
            get_snapshot,
            get_media_snapshot,
            probe_media,
            create_media_task,
            cancel_media_task,
            retry_media_task,
            open_media_output,
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
            let media = ffmpeg::runner::MediaService::new(
                app.handle().clone(),
                &service.data,
                &service.resources,
            )
            .map_err(std::io::Error::other)?;
            app.manage(media.clone());
            media.start();
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
                    "exit" => match app.state::<Arc<ffmpeg::runner::MediaService>>().shutdown() {
                        Ok(()) => {
                            app.state::<Arc<Service>>().shutdown();
                            app.exit(0);
                        }
                        Err(error) => native::message(&error),
                    },
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
                let _ = app.state::<Arc<ffmpeg::runner::MediaService>>().shutdown();
                app.state::<Arc<Service>>().shutdown();
            }
        }),
        Err(error) => native::message(&format!("软件启动失败：{error}")),
    }
}
