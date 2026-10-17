use crate::{
    domain::AppResult,
    native::{self, Job},
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
#[derive(Default)]
pub struct Control {
    cancelled: AtomicBool,
    job: Mutex<Option<Arc<Job>>>,
}
impl Control {
    #[cfg(test)]
    pub(crate) fn has_job(&self) -> bool {
        self.job.lock().unwrap().is_some()
    }
    pub fn check(&self) -> AppResult<()> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err("分析已取消".into())
        } else {
            Ok(())
        }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(job) = self.job.lock() {
            if let Some(job) = job.as_ref() {
                let _ = job.terminate();
            }
        }
    }
    pub fn capture(
        &self,
        exe: &Path,
        args: &[String],
        directory: Option<&Path>,
        timeout: Duration,
    ) -> AppResult<Vec<u8>> {
        self.check()?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut timer = None;
        let mut registered = Ok(());
        let result = native::capture_bytes_in(exe, args, 32 * 1024 * 1024, directory, |job| {
            registered = self
                .job
                .lock()
                .map_err(|_| "分析进程状态不可用".to_owned())
                .map(|mut active| {
                    *active = Some(job.clone());
                    if self.check().is_err() {
                        let _ = job.terminate();
                    }
                });
            timer = Some(std::thread::spawn(move || {
                if rx.recv_timeout(timeout).is_err() {
                    let _ = job.terminate();
                }
            }));
        });
        let _ = tx.send(());
        if let Some(t) = timer {
            let _ = t.join();
        }
        if let Ok(mut job) = self.job.lock() {
            *job = None;
        }
        registered?;
        self.check()?;
        result.map_err(|_| "本地媒体处理失败或超时，请检查组件、源文件及可用磁盘空间".into())
    }
}
#[derive(Default)]
pub struct AiRuntime {
    pub active: Option<(String, Arc<Control>)>,
    pub exiting: bool,
}
