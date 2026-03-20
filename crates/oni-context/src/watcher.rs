use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Watches a project directory for file changes and sends changed paths
/// through a channel. Designed to run in a background thread.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<PathBuf>,
}

impl FileWatcher {
    /// Start watching `dir` recursively. Returns a FileWatcher that yields
    /// changed file paths via `poll()`.
    pub fn start(dir: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();
        let dir_owned = dir.to_path_buf();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        // Skip hidden files, .git, node_modules, etc.
                        let path_str = path.to_string_lossy();
                        if path_str.contains("/.git/")
                            || path_str.contains("/node_modules/")
                            || path_str.contains("/target/")
                            || path_str.contains("/.oni/")
                        {
                            continue;
                        }
                        let _ = tx.send(path);
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )?;

        watcher.watch(&dir_owned, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Non-blocking poll for changed paths. Returns all paths that changed
    /// since the last poll.
    pub fn poll(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        while let Ok(path) = self.rx.try_recv() {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        paths
    }
}
