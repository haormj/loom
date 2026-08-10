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
                log::warn!(
                    "dashboard watcher: notify unavailable ({}), falling back to polling",
                    e
                );
                poll_loop(project_root_clone, tx);
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

fn collect_poll_targets(loom_dir: &PathBuf) -> Vec<PathBuf> {
    let mut targets = Vec::new();

    let status = loom_dir.join("status.json");
    if status.exists() {
        targets.push(status);
    }

    if let Ok(entries) = std::fs::read_dir(loom_dir.join("deliveries")) {
        for entry in entries.flatten() {
            let index = entry.path().join("index.json");
            if index.exists() {
                targets.push(index);
            }
        }
    }

    let state_dir = loom_dir.join("deployment").join("state");
    if let Ok(entries) = std::fs::read_dir(&state_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("json") {
                targets.push(p);
            }
        }
    }

    targets
}

fn poll_loop(project_root: PathBuf, tx: broadcast::Sender<DashboardEvent>) {
    let loom_dir = project_root.join(".loom");
    let poll_interval = Duration::from_secs(5);

    let mut last_mtimes: std::collections::HashMap<PathBuf, std::time::SystemTime> =
        std::collections::HashMap::new();

    let targets = collect_poll_targets(&loom_dir);
    for path in &targets {
        if let Ok(mtime) = std::fs::metadata(path).and_then(|m| m.modified()) {
            last_mtimes.insert(path.clone(), mtime);
        }
    }

    loop {
        std::thread::sleep(poll_interval);

        if tx.receiver_count() == 0 {
            log::debug!("dashboard watcher (poll): no receivers, stopping");
            return;
        }

        let targets = collect_poll_targets(&loom_dir);
        let mut changed: Vec<String> = Vec::new();
        let mut event_type = "ignore";

        for path in &targets {
            let current = match std::fs::metadata(path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let prev = last_mtimes.insert(path.clone(), current);
            if prev != Some(current) {
                let rel = path
                    .strip_prefix(&project_root)
                    .map(|s| s.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());
                let ps = rel.as_str();
                if event_type == "ignore" {
                    if ps.contains("status.json") {
                        event_type = "status";
                    } else if ps.contains("deliveries") && ps.contains("index.json") {
                        event_type = "delivery";
                    } else if ps.contains("deployment") && ps.contains("state") {
                        event_type = "deploy";
                    }
                }
                changed.push(rel);
            }
        }

        if event_type != "ignore" && !changed.is_empty() {
            let event = DashboardEvent {
                event_type: event_type.to_string(),
                data: serde_json::json!({ "paths": changed }),
            };
            if tx.send(event).is_err() {
                log::debug!("dashboard watcher (poll): no receivers, stopping");
                return;
            }
        }
    }
}
