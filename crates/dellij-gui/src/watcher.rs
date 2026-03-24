//! File-system watcher: detects changes to `.dellij/status/*.json`
//! and pushes reload events back into the GPUI model.
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{Event as FsEvent, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};

pub struct StatusWatcher {
    _watcher: RecommendedWatcher,
}

impl StatusWatcher {
    /// Spawn a background watcher on `status_dir`.
    /// Calls `on_change` on every file-system event (debounced ~200 ms).
    pub fn spawn(
        status_dir: impl AsRef<Path>,
        mut on_change: impl FnMut() + Send + 'static,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel::<NotifyResult<FsEvent>>();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(status_dir.as_ref(), RecursiveMode::NonRecursive)?;

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(_) => {
                        // Drain burst events with a small debounce
                        thread::sleep(Duration::from_millis(200));
                        while rx.try_recv().is_ok() {}
                        on_change();
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self { _watcher: watcher })
    }
}
