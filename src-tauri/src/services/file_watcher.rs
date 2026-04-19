use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const SUPPORTED_EXTENSIONS: &[&str] = &["pes", "dst", "jef", "vp3", "pdf", "png", "jpg", "jpeg", "bmp"];
const DEBOUNCE_MS: u64 = 500;

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Clone, serde::Serialize)]
struct FsEventPayload {
    paths: Vec<String>,
}

pub struct WatcherState {
    _watcher: RecommendedWatcher,
}

/// Managed state holding the active watcher (if any).
pub struct WatcherHolder(pub Mutex<Option<WatcherState>>);

pub fn start_watcher(
    watch_path: &str,
    app_handle: &AppHandle,
) -> Result<WatcherState, String> {
    let watch_dir = PathBuf::from(watch_path);
    if !watch_dir.is_dir() {
        return Err(format!("Watch path is not a directory: {}", watch_path));
    }

    // Audit Wave 2 perf: notify's Watcher requires `mpsc::Sender` (unbounded),
    // so the upper bound is enforced downstream via the HashSet flush cap
    // below — once the accumulator reaches 500 entries it's flushed even if
    // the debounce window hasn't elapsed.
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch directory: {e}"))?;

    // Spawn debounce thread
    let handle = app_handle.clone();
    std::thread::spawn(move || {
        let mut new_files: HashSet<String> = HashSet::new();
        let mut removed_files: HashSet<String> = HashSet::new();
        let mut last_flush = Instant::now();

        loop {
            match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                Ok(Ok(event)) => {
                    for path in &event.paths {
                        if !is_supported_file(path) {
                            continue;
                        }
                        let path_str = path.to_string_lossy().to_string();
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                new_files.insert(path_str);
                            }
                            EventKind::Remove(_) => {
                                removed_files.insert(path_str);
                            }
                            _ => {}
                        }
                    }
                    // Audit Wave 2 perf: cap accumulator size so a sustained
                    // burst flushes proactively even before the timeout.
                    if new_files.len() >= 500 || removed_files.len() >= 500 {
                        if !new_files.is_empty() {
                            let _ = handle.emit(
                                "fs:new-files",
                                FsEventPayload { paths: new_files.drain().collect() },
                            );
                        }
                        if !removed_files.is_empty() {
                            let _ = handle.emit(
                                "fs:files-removed",
                                FsEventPayload { paths: removed_files.drain().collect() },
                            );
                        }
                        last_flush = Instant::now();
                    }
                }
                Ok(Err(e)) => {
                    log::warn!("Watcher error: {e}");
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Flush accumulated events
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Flush remaining events before exiting
                    if !new_files.is_empty() {
                        let _ = handle.emit(
                            "fs:new-files",
                            FsEventPayload {
                                paths: new_files.drain().collect(),
                            },
                        );
                    }
                    if !removed_files.is_empty() {
                        let _ = handle.emit(
                            "fs:files-removed",
                            FsEventPayload {
                                paths: removed_files.drain().collect(),
                            },
                        );
                    }
                    break;
                }
            }

            // Flush if debounce window has passed and we have events
            if last_flush.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                if !new_files.is_empty() {
                    let _ = handle.emit(
                        "fs:new-files",
                        FsEventPayload {
                            paths: new_files.drain().collect(),
                        },
                    );
                }
                if !removed_files.is_empty() {
                    let _ = handle.emit(
                        "fs:files-removed",
                        FsEventPayload {
                            paths: removed_files.drain().collect(),
                        },
                    );
                }
                last_flush = Instant::now();
            }
        }
    });

    Ok(WatcherState { _watcher: watcher })
}

#[tauri::command]
pub fn watcher_start(
    path: String,
    app_handle: AppHandle,
    holder: tauri::State<'_, WatcherHolder>,
) -> Result<(), String> {
    let mut guard = holder
        .0
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;

    // Stop existing watcher if any
    *guard = None;

    // Expand ~ to home directory
    let expanded = if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&path[2..]).to_string_lossy().to_string()
        } else {
            path.clone()
        }
    } else {
        path.clone()
    };

    let state = start_watcher(&expanded, &app_handle)?;
    *guard = Some(state);
    Ok(())
}

#[tauri::command]
pub fn watcher_stop(
    holder: tauri::State<'_, WatcherHolder>,
) -> Result<(), String> {
    let mut guard = holder
        .0
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn watcher_get_status(
    holder: tauri::State<'_, WatcherHolder>,
) -> Result<bool, String> {
    let guard = holder
        .0
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    Ok(guard.is_some())
}
