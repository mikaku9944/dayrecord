use dayrecord_app_lib::init_state;

#[test]
fn init_state_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("LOCALAPPDATA", dir.path());
    let state = init_state().expect("init");
    assert!(state.orchestrator.is_recording());
}
