use std::path::PathBuf;

use dashboard::reader::{
    list_deliveries, list_knowledge_sources, read_audit_records, read_delivery_index,
    read_deploy_state, read_project,
};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/rust/dashboard/fixtures/minimal_project")
}

#[test]
fn read_project_returns_status_when_initialized() {
    let snapshot = read_project(&fixture_root());
    assert!(snapshot.initialized);
    assert!(snapshot.status.is_some());
    let status = snapshot.status.as_ref().unwrap();
    assert_eq!(status.active_delivery_id.as_deref(), Some("del_test"));
}

#[test]
fn read_project_returns_uninitialized_when_no_loom_dir() {
    let tmp = std::env::temp_dir().join("loom_dashboard_test_empty");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let snapshot = read_project(&tmp);
    assert!(!snapshot.initialized);
    assert!(snapshot.status.is_none());
}

#[test]
fn list_deliveries_returns_all_delivery_indices() {
    let deliveries = list_deliveries(&fixture_root());
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].delivery_id, "del_test");
    assert_eq!(deliveries[0].status, "executing");
}

#[test]
fn read_delivery_index_returns_phases() {
    let delivery = read_delivery_index(&fixture_root(), "del_test").unwrap();
    assert_eq!(delivery.phases.len(), 1);
    assert_eq!(delivery.phases[0].phase_id, "ph_01");
    assert!(delivery.phases[0].latest_refs.contains_key("taskPlanRun"));
}

#[test]
fn read_deploy_state_returns_empty_when_no_deployment() {
    let snapshot = read_deploy_state(&fixture_root());
    assert!(!snapshot.prepared);
    assert!(snapshot.state.is_none());
    assert!(snapshot.log_tail.is_empty());
}

#[test]
fn list_knowledge_sources_does_not_panic_without_loom_home() {
    let sources = list_knowledge_sources();
    let _ = sources.len();
}

#[test]
fn read_audit_records_returns_empty_when_no_metrics() {
    let records = read_audit_records(&fixture_root(), 50);
    assert!(records.is_empty());
}
