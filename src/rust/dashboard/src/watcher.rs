use std::path::PathBuf;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardEvent {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub data: serde_json::Value,
}

const DEBOUNCE_MS: u64 = 100;

pub fn start_watcher(project_root: PathBuf) -> broadcast::Receiver<DashboardEvent> {
    let (tx, rx) = broadcast::channel::<DashboardEvent>(64);

    let loom_dir = project_root.join(".loom");
    let project_root_clone = project_root.clone();

    std::thread::spawn(move || {
        if !loom_dir.exists() {
            log::warn!("dashboard watcher: .loom/ does not exist, skipping");
            return;
        }

        let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();

        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                let _ = notify_tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(DEBOUNCE_MS)),
        ) {
            Ok(w) => w,
            Err(e) => {
                log::warn!("dashboard watcher: failed to create watcher: {}", e);
                return;
            }
        };

        let watch_paths = [
            loom_dir.join("status.json"),
            loom_dir.join("deliveries"),
            loom_dir.join("deployment").join("state"),
            loom_dir.join("deployment").join("logs").join("local.log"),
        ];

        for path in &watch_paths {
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    log::warn!("dashboard watcher: cannot watch {:?}: {}", path, e);
                }
            }
        }

        let mut last_emit = std::time::Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);

        for event_result in notify_rx {
            match event_result {
                Ok(event) => {
                    if last_emit.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
                        continue;
                    }
                    last_emit = std::time::Instant::now();

                    let event_type = classify_event(&event.kind, &event.paths);
                    if event_type == "ignore" {
                        continue;
                    }

                    let dashboard_event = DashboardEvent {
                        event_type: event_type.to_string(),
                        data: serde_json::json!({
                            "paths": event.paths.iter().map(|p| {
                                p.strip_prefix(&project_root_clone)
                                    .map(|s| s.display().to_string())
                                    .unwrap_or_else(|_| p.display().to_string())
                            }).collect::<Vec<_>>(),
                        }),
                    };

                    if tx.send(dashboard_event).is_err() {
                        log::debug!("dashboard watcher: no receivers, stopping");
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("dashboard watcher: event error: {}", e);
                }
            }
        }
    });

    rx
}

fn classify_event(kind: &EventKind, paths: &[PathBuf]) -> &'static str {
    if !matches!(kind, EventKind::Modify(_) | EventKind::Create(_)) {
        return "ignore";
    }
    for path in paths {
        let path_str = path.to_string_lossy();
        if path_str.contains("status.json") {
            return "status";
        }
        if path_str.contains("deliveries") && path_str.contains("index.json") {
            return "delivery";
        }
        if path_str.contains("tasks") && path_str.contains("runs") {
            return "tasks";
        }
        if path_str.contains("reviews") {
            return "reviews";
        }
        if path_str.contains("deployment") && path_str.contains("state") {
            return "deploy";
        }
        if path_str.contains("local.log") {
            return "deploy_logs";
        }
    }
    "ignore"
}
