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
pub struct Control {
    cancelled: AtomicBool,
    signal: tokio::sync::watch::Sender<bool>,
    job: Mutex<Option<Arc<Job>>>,
}
impl Default for Control {
    fn default() -> Self {
        let (signal, _) = tokio::sync::watch::channel(false);
        Self {
            cancelled: AtomicBool::new(false),
            signal,
            job: Mutex::new(None),
        }
    }
}
impl Control {
    pub async fn reply(
        &self,
        future: impl std::future::Future<Output = AppResult<super::client::ChatReply>>,
    ) -> AppResult<super::client::ChatReply> {
        self.check()?;
        let mut receiver = self.signal.subscribe();
        tokio::select! {biased;result=future=>result,_=receiver.wait_for(|cancelled|*cancelled)=>Err("分析已取消".into())}
    }

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
        self.signal.send_replace(true);
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
