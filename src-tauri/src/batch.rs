use crate::{
    domain::{self, AppResult},
    models::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    Pin,
    Copy,
    Remove,
}
impl BatchAction {
    pub fn command(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Cancel => "cancel",
            Self::Retry => "retry",
            Self::Pin => "pin",
            Self::Copy => "copy",
            Self::Remove => "remove",
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Failed {
    pub id: String,
    pub error: String,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct BatchResult {
    pub succeeded: Vec<String>,
    pub skipped: Vec<Skipped>,
    pub failed: Vec<Failed>,
}

// Match the existing single-task buttons, which are narrower than the legacy IPC.
pub fn skip_reason(status: &TaskStatus, action: BatchAction) -> Option<&'static str> {
    use BatchAction::*;
    use TaskStatus::*;
    match action {
        Pause if matches!(status, Queued | Resolving | Downloading) => None,
        Resume if *status == Paused => None,
        Cancel
            if matches!(
                status,
                Queued | Resolving | Downloading | Processing | Paused
            ) =>
        {
            None
        }
        Retry if matches!(status, Failed | Cancelled) => None,
        Pin if matches!(status, Queued | Paused) => None,
        Remove if matches!(status, Completed | Failed | Cancelled) => None,
        Copy => None,
        Pause if *status == Processing => Some("合并或转换阶段不可暂停，可取消或等待完成"),
        Pause => Some("当前任务不在排队或下载阶段"),
        Resume => Some("只有已暂停的任务可以恢复"),
        Cancel => Some("已完成、失败或已取消的任务无需取消"),
        Retry => Some("只有失败或已取消的任务可以重试"),
        Pin => Some("只有排队中或已暂停的任务可以置顶"),
        Remove => Some("请先取消任务，等待内核停止后再删除历史记录"),
    }
}
pub fn prepare(
    tasks: &[DownloadTask],
    ids: &[String],
    action: BatchAction,
    running: &[String],
) -> AppResult<BatchResult> {
    if ids.is_empty() || ids.len() > 1000 {
        return Err("请选择 1–1000 个任务".into());
    }
    let mut result = BatchResult::default();
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            continue;
        }
        let reason = match tasks.iter().find(|task| task.id == *id) {
            None => Some("任务不存在，可能已被删除"),
            Some(task) => skip_reason(&task.status, action).or_else(|| {
                (action == BatchAction::Remove && running.contains(id))
                    .then_some("请先取消任务，等待内核停止后再移除记录")
            }),
        };
        if let Some(reason) = reason {
            result.skipped.push(Skipped {
                id: id.clone(),
                reason: reason.into(),
            });
        } else {
            result.succeeded.push(id.clone());
        }
    }
    Ok(result)
}
pub fn execute(
    mut result: BatchResult,
    mut apply: impl FnMut(&str) -> AppResult<()>,
    finish: impl FnOnce(),
) -> BatchResult {
    for id in std::mem::take(&mut result.succeeded) {
        match apply(&id) {
            Ok(()) => result.succeeded.push(id),
            Err(error) => result.failed.push(Failed {
                id,
                error: domain::redact(&error),
            }),
        }
    }
    complete(result, finish)
}
pub fn complete(result: BatchResult, finish: impl FnOnce()) -> BatchResult {
    finish();
    result
}
pub fn remove_record(tasks: &mut Vec<DownloadTask>, id: &str, running: &[String]) -> AppResult<()> {
    if running.iter().any(|entry| entry == id) {
        return Err("请先取消任务，等待内核停止后再移除记录".into());
    }
    tasks.retain(|task| task.id != id);
    Ok(())
}
pub fn records(
    disk: &mut DiskState,
    ids: &[String],
    action: BatchAction,
    running: &[String],
    save: impl FnOnce(&DiskState) -> AppResult<()>,
) -> AppResult<BatchResult> {
    if !matches!(action, BatchAction::Pin | BatchAction::Remove) {
        return Err("不是记录操作".into());
    }
    let mut result = prepare(&disk.tasks, ids, action, running)?;
    if result.succeeded.is_empty() {
        return Ok(result);
    }
    let mut next = disk.clone();
    if action == BatchAction::Remove {
        for id in &result.succeeded {
            remove_record(&mut next.tasks, id, running)?;
        }
    } else {
        let order = next
            .tasks
            .iter()
            .map(|task| task.queue_order)
            .max()
            .unwrap_or(0)
            .checked_add(result.succeeded.len() as u64)
            .ok_or("队列排序已达到上限")?;
        let mut pinned = next
            .tasks
            .iter_mut()
            .filter(|task| result.succeeded.contains(&task.id))
            .collect::<Vec<_>>();
        pinned.sort_by(|a, b| {
            b.queue_order
                .cmp(&a.queue_order)
                .then(a.created_at.cmp(&b.created_at))
        });
        for (index, task) in pinned.into_iter().enumerate() {
            task.queue_order = order - index as u64;
        }
    }
    match save(&next) {
        Ok(()) => *disk = next,
        Err(error) => {
            let error = domain::redact(&error);
            result
                .failed
                .extend(
                    std::mem::take(&mut result.succeeded)
                        .into_iter()
                        .map(|id| Failed {
                            id,
                            error: error.clone(),
                        }),
                );
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tasks() -> Vec<DownloadTask> {
        [
            TaskStatus::Queued,
            TaskStatus::Resolving,
            TaskStatus::Downloading,
            TaskStatus::Processing,
            TaskStatus::Paused,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, status)| DownloadTask {
            id: i.to_string(),
            status,
            ..Default::default()
        })
        .collect()
    }
    #[test]
    fn batch_matches_every_single_button_state_and_explains_skips() {
        let cases = [
            (BatchAction::Pause, vec![0, 1, 2]),
            (BatchAction::Resume, vec![4]),
            (BatchAction::Cancel, vec![0, 1, 2, 3, 4]),
            (BatchAction::Retry, vec![6, 7]),
            (BatchAction::Pin, vec![0, 4]),
            (BatchAction::Remove, vec![5, 6, 7]),
            (BatchAction::Copy, vec![0, 1, 2, 3, 4, 5, 6, 7]),
        ];
        for (action, allowed) in cases {
            for (i, task) in tasks().iter().enumerate() {
                assert_eq!(
                    skip_reason(&task.status, action).is_none(),
                    allowed.contains(&i),
                    "{action:?} {:?}",
                    task.status
                );
                if !allowed.contains(&i) {
                    assert!(!skip_reason(&task.status, action).unwrap().is_empty());
                }
            }
        }
        assert!(skip_reason(&TaskStatus::Processing, BatchAction::Pause)
            .unwrap()
            .contains("合并或转换"));
        assert!(skip_reason(&TaskStatus::Downloading, BatchAction::Remove)
            .unwrap()
            .contains("请先取消"));
    }
    #[test]
    fn batch_validates_ids_and_schedules_once_with_per_item_failures() {
        let tasks = tasks();
        assert!(prepare(&tasks, &[], BatchAction::Pause, &[]).is_err());
        assert!(prepare(&tasks, &vec!["0".into(); 1001], BatchAction::Pause, &[]).is_err());
        let ids = ["0", "0", "2", "3", "missing"].map(String::from);
        let planned = prepare(&tasks, &ids, BatchAction::Pause, &[]).unwrap();
        let mut calls = Vec::new();
        let mut schedules = 0;
        let result = execute(
            planned,
            |id| {
                calls.push(id.to_string());
                if id == "2" {
                    Err("测试写入失败".into())
                } else {
                    Ok(())
                }
            },
            || schedules += 1,
        );
        assert_eq!(calls, vec!["0", "2"]);
        assert_eq!(schedules, 1);
        assert_eq!(result.succeeded, vec!["0"]);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.skipped.len(), 2);
    }
    #[test]
    fn records_write_once_and_never_remove_downloads_or_temporary_files() {
        let dir = std::env::temp_dir().join(format!("batch-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".video-downloader/test")).unwrap();
        let file = dir.join("movie.mp4");
        let partial = dir.join(".video-downloader/test/movie.part");
        std::fs::write(&file, b"existing").unwrap();
        std::fs::write(&partial, b"partial").unwrap();
        let mut disk = DiskState {
            tasks: tasks(),
            ..Default::default()
        };
        disk.tasks[5].files = vec![file.to_string_lossy().into_owned()];
        let mut writes = 0;
        let result = records(
            &mut disk,
            &["5".into(), "6".into(), "7".into(), "0".into()],
            BatchAction::Remove,
            &[],
            |_| {
                writes += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(writes, 1);
        assert_eq!(result.succeeded.len(), 3);
        assert_eq!(disk.tasks.len(), 5);
        assert!(file.is_file() && partial.is_file());
        std::fs::remove_file(file).unwrap();
        std::fs::remove_file(partial).unwrap();
        std::fs::remove_dir(dir.join(".video-downloader/test")).unwrap();
        std::fs::remove_dir(dir.join(".video-downloader")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn repeated_pin_preserves_the_current_order_of_selected_and_other_tasks() {
        let mut disk = DiskState {
            tasks: (0..4)
                .map(|index| DownloadTask {
                    id: index.to_string(),
                    status: TaskStatus::Queued,
                    created_at: index,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        records(&mut disk, &["2".into()], BatchAction::Pin, &[], |_| Ok(())).unwrap();
        assert_eq!(
            crate::engine::queued_ids(&disk.tasks, 4, &[]),
            ["2", "0", "1", "3"]
        );
        records(
            &mut disk,
            &["0".into(), "2".into()],
            BatchAction::Pin,
            &[],
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(
            crate::engine::queued_ids(&disk.tasks, 4, &[]),
            ["2", "0", "1", "3"]
        );
    }
    #[test]
    fn pin_preserves_selected_order_and_old_records_keep_original_order() {
        let old: DownloadTask = serde_json::from_str(
            r#"{"id":"old","status":"queued","request":{"url":"https://example.com/video"}}"#,
        )
        .unwrap();
        assert_eq!(old.queue_order, 0);
        let mut disk = DiskState {
            tasks: vec![
                old,
                DownloadTask {
                    id: "one".into(),
                    ..Default::default()
                },
                DownloadTask {
                    id: "two".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            crate::engine::queued_ids(&disk.tasks, 2, &[]),
            vec!["old", "one"]
        );
        records(
            &mut disk,
            &["two".into(), "one".into()],
            BatchAction::Pin,
            &[],
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(
            crate::engine::queued_ids(&disk.tasks, 3, &[]),
            vec!["one", "two", "old"]
        );
        let before = serde_json::to_value(&disk).unwrap();
        let result = records(&mut disk, &["old".into()], BatchAction::Remove, &[], |_| {
            Err("不可写".into())
        })
        .unwrap();
        assert_eq!(serde_json::to_value(&disk).unwrap(), before);
        assert!(result.succeeded.is_empty());
    }
}
