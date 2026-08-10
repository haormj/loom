use std::path::PathBuf;
use std::time::Duration;

use dashboard::watcher::start_watcher;

#[test]
fn watcher_emits_status_event_on_file_change() {
    let tmp = tempfile::tempdir().unwrap();
    let loom_dir = tmp.path().join(".loom");
    std::fs::create_dir_all(&loom_dir).unwrap();
    let status_file = loom_dir.join("status.json");
    std::fs::write(
        &status_file,
        r#"{"schemaVersion":1,"activeDeliveryId":null,"deliveries":[],"updatedAt":"x"}"#,
    )
    .unwrap();

    let mut rx = start_watcher(tmp.path().to_path_buf());

    // Modify the file to trigger an event
    std::thread::sleep(Duration::from_millis(200));
    std::fs::write(
        &status_file,
        r#"{"schemaVersion":1,"activeDeliveryId":"del_1","deliveries":[],"updatedAt":"y"}"#,
    )
    .unwrap();

    // Wait for event (with timeout)
    let result = rx.recv_timeout(Duration::from_secs(5));
    assert!(result.is_ok(), "should receive an event within 5 seconds");
    let event = result.unwrap();
    assert!(
        event.event_type == "status" || event.event_type == "deploy_logs",
        "event type should be classified, got: {}",
        event.event_type
    );
}
