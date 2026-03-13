use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::Disks;
use tauri::{AppHandle, Emitter};

const POLL_INTERVAL_MS: u64 = 3000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    pub name: String,
    pub mount_point: String,
    pub total_space_bytes: u64,
    pub free_space_bytes: u64,
}

pub struct UsbMonitorState {
    _shutdown_tx: mpsc::Sender<()>,
}

pub struct UsbMonitorHolder(pub Mutex<Option<UsbMonitorState>>);

fn detect_usb_devices() -> Vec<UsbDevice> {
    let disks = Disks::new_with_refreshed_list();
    let mut devices = Vec::new();

    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy().to_string();

        let is_usb = if cfg!(target_os = "linux") {
            (mount.starts_with("/media/")
                || mount.starts_with("/run/media/")
                || (mount.starts_with("/mnt/") && mount != "/mnt"))
                && disk.is_removable()
        } else if cfg!(target_os = "macos") {
            mount.starts_with("/Volumes/") && disk.is_removable()
        } else if cfg!(target_os = "windows") {
            disk.is_removable()
        } else {
            disk.is_removable()
        };

        if is_usb {
            let name = disk.name().to_string_lossy().to_string();
            devices.push(UsbDevice {
                name: if name.is_empty() {
                    "USB".to_string()
                } else {
                    name
                },
                mount_point: mount,
                total_space_bytes: disk.total_space(),
                free_space_bytes: disk.available_space(),
            });
        }
    }

    devices
}

pub fn start_usb_monitor(app_handle: &AppHandle) -> Result<UsbMonitorState, String> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    let handle = app_handle.clone();

    std::thread::spawn(move || {
        let mut known: HashMap<String, UsbDevice> = HashMap::new();

        // Initial detection
        for dev in detect_usb_devices() {
            known.insert(dev.mount_point.clone(), dev);
        }

        loop {
            match shutdown_rx.recv_timeout(Duration::from_millis(POLL_INTERVAL_MS)) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }

            let current = detect_usb_devices();
            let current_map: HashMap<String, UsbDevice> = current
                .into_iter()
                .map(|d| (d.mount_point.clone(), d))
                .collect();

            // Detect newly connected devices
            for (mount, device) in &current_map {
                if !known.contains_key(mount) {
                    let _ = handle.emit("usb:connected", device.clone());
                }
            }

            // Detect disconnected devices
            for (mount, device) in &known {
                if !current_map.contains_key(mount) {
                    let _ = handle.emit("usb:disconnected", device.clone());
                }
            }

            known = current_map;
        }
    });

    Ok(UsbMonitorState {
        _shutdown_tx: shutdown_tx,
    })
}

#[tauri::command]
pub fn get_usb_devices() -> Vec<UsbDevice> {
    detect_usb_devices()
}

#[tauri::command]
pub fn usb_monitor_start(
    app_handle: AppHandle,
    holder: tauri::State<'_, UsbMonitorHolder>,
) -> Result<(), String> {
    let mut guard = holder
        .0
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;

    // Stop existing monitor: dropping the old state sends shutdown signal
    // via the dropped _shutdown_tx. The Mutex guard is held for the entire
    // duration, preventing concurrent callers from spawning a second thread.
    *guard = None;

    let state = start_usb_monitor(&app_handle)?;
    *guard = Some(state);
    Ok(())
}

#[tauri::command]
pub fn usb_monitor_stop(
    holder: tauri::State<'_, UsbMonitorHolder>,
) -> Result<(), String> {
    let mut guard = holder
        .0
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    *guard = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_usb_devices_does_not_panic() {
        let devices = detect_usb_devices();
        // On CI/dev machines there may be 0 USB devices — just ensure no crash.
        // Upper bound of 100 is a sanity check: no real system has >100 USB drives.
        assert!(devices.len() < 100);
    }
}
