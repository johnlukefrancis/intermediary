// Path: src-tauri/src/lib/crosstalk/observer.rs
// Description: One filesystem-notified feed worker tied to the native app lifetime

use super::{feed::Feed, CrosstalkState, LABEL};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};
use tauri::{AppHandle, Emitter, Manager};

pub struct Observer {
    watcher: RecommendedWatcher,
    cancel: Arc<AtomicBool>,
    wake: mpsc::SyncSender<()>,
    thread: thread::JoinHandle<()>,
}

impl Observer {
    pub fn stop(self) {
        self.cancel.store(true, Ordering::Release);
        drop(self.watcher);
        let _ = self.wake.try_send(());
        let _ = self.thread.join();
    }
}

pub fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<CrosstalkState>();
    let mut owner = state
        .observer
        .lock()
        .map_err(|_| "Crosstalk observer lock failed")?;
    if owner.is_some() {
        return Ok(());
    }
    let root = std::env::var_os("CROSSTALK_ROOT")
        .map(PathBuf::from)
        .or_else(|| dirs::data_local_dir().map(|p| p.join("crosstalk")))
        .ok_or("Could not resolve Crosstalk local data")?;
    let path = root.join("events.jsonl");
    let mut feed =
        Feed::at_end(&path).map_err(|e| format!("Could not open Crosstalk feed: {e}"))?;
    let (wake, inbox) = mpsc::sync_channel(1);
    let event_wake = wake.clone();
    let event_path = path.clone();
    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
            Ok(event)
                if !matches!(event.kind, notify::EventKind::Access(_))
                    && event.paths.iter().any(|p| p == &event_path) =>
            {
                let _ = event_wake.try_send(());
            }
            Err(error) => super::report(&format!("Crosstalk watch failed: {error}")),
            _ => {}
        })
        .map_err(|e| format!("Could not create Crosstalk watch: {e}"))?;
    watcher
        .watch(&root, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Could not watch Crosstalk event directory: {e}"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let app = app.clone();
    let worker = thread::Builder::new()
        .name("crosstalk-feed".into())
        .spawn(move || {
            while inbox.recv().is_ok() {
                if worker_cancel.load(Ordering::Acquire) {
                    break;
                }
                match feed.read(&path) {
                    Ok(batch) => {
                        for turn in batch.turns {
                            if worker_cancel.load(Ordering::Acquire) {
                                break;
                            }
                            if let Err(error) = app.emit_to("main", "crosstalk-turn", turn) {
                                super::report(&format!(
                                    "Could not publish Crosstalk turn: {error}"
                                ));
                            }
                        }
                        let state = app.state::<CrosstalkState>();
                        let Ok(view) = state.view.lock() else {
                            break;
                        };
                        if !view.presentation.active || !view.window_visible || !view.ready {
                            continue;
                        }
                        for mut message in batch.messages {
                            if worker_cancel.load(Ordering::Acquire) {
                                break;
                            }
                            message.cwd = sender_cwd(&root, &message.from);
                            if let Err(error) = app.emit_to(LABEL, "crosstalk-message", message) {
                                super::report(&format!(
                                    "Could not display Crosstalk message: {error}"
                                ));
                            }
                        }
                    }
                    Err(error) => super::report(&format!("Could not read Crosstalk feed: {error}")),
                }
            }
        })
        .map_err(|e| format!("Could not start Crosstalk observer: {e}"))?;
    let _ = wake.try_send(());
    *owner = Some(Observer {
        watcher,
        cancel,
        wake,
        thread: worker,
    });
    Ok(())
}

fn sender_cwd(root: &Path, sender: &str) -> Option<String> {
    if sender.len() > 240
        || !sender
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@#-".contains(&b))
    {
        return None;
    }
    #[derive(serde::Deserialize)]
    struct Registration {
        cwd: String,
    }
    let file = std::fs::File::open(root.join("agents").join(format!("{sender}.json"))).ok()?;
    let mut bytes = Vec::new();
    file.take(16 * 1024).read_to_end(&mut bytes).ok()?;
    serde_json::from_slice::<Registration>(&bytes)
        .ok()
        .map(|r| r.cwd)
}
