// Path: src-tauri/src/lib/agent/install_tests.rs
// Description: Agent bundle installation and packaged-runtime identity regression tests

use super::{
    resolve_launch_bundle, AgentBundleInstallState, AGENT_BUNDLE_DIR, AGENT_INSTALL_DIR,
    AGENT_VERSION_FILE, HOST_AGENT_BINARY_FILE, WSL_AGENT_BINARY_FILE,
};
use std::fs;
use std::path::Path;

#[test]
fn prefer_installed_reinstalls_when_app_local_binary_is_stale() {
    let temp = tempfile::tempdir().expect("tempdir");
    let resource_dir = temp.path().join("resources");
    let app_local_data = temp.path().join("app_local");
    let bundle_dir = resource_dir.join(AGENT_BUNDLE_DIR);
    let install_dir = app_local_data.join(AGENT_INSTALL_DIR);

    write_agent_bundle(&bundle_dir, "resource-host", "resource-wsl");
    write_installed_agent_bundle(&install_dir, "stale-host", "stale-wsl");

    let resolved =
        resolve_launch_bundle(&resource_dir, &app_local_data, true).expect("resolved bundle");

    assert_eq!(resolved.install_state, AgentBundleInstallState::Installed);
    assert_eq!(resolved.bundle.agent_dir_host, install_dir);
    assert_ne!(
        fs::read(resolved.bundle.host_agent_binary_host).expect("host agent"),
        b"stale-host"
    );
    if let Some(wsl_agent) = resolved.bundle.wsl_agent_binary_host {
        assert_eq!(
            fs::read_to_string(wsl_agent).expect("wsl agent"),
            "resource-wsl"
        );
    }
}

#[test]
fn prefer_installed_reports_current_when_app_local_bundle_matches() {
    let temp = tempfile::tempdir().expect("tempdir");
    let resource_dir = temp.path().join("resources");
    let app_local_data = temp.path().join("app_local");
    let bundle_dir = resource_dir.join(AGENT_BUNDLE_DIR);
    write_agent_bundle(&bundle_dir, "same-host", "same-wsl");

    let first =
        resolve_launch_bundle(&resource_dir, &app_local_data, false).expect("initial install");
    assert_eq!(first.install_state, AgentBundleInstallState::Installed);

    let resolved =
        resolve_launch_bundle(&resource_dir, &app_local_data, true).expect("resolved bundle");

    assert_eq!(resolved.install_state, AgentBundleInstallState::Current);
    assert_eq!(resolved.bundle.agent_dir_host, first.bundle.agent_dir_host);
}

fn write_agent_bundle(bundle_dir: &Path, host_binary: &str, wsl_binary: &str) {
    fs::create_dir_all(bundle_dir).expect("bundle dir");
    fs::write(
        bundle_dir.join(AGENT_VERSION_FILE),
        r#"{"version":"1.0.0"}"#,
    )
    .expect("bundle version");
    fs::write(bundle_dir.join(HOST_AGENT_BINARY_FILE), host_binary).expect("bundle host");
    fs::write(bundle_dir.join(WSL_AGENT_BINARY_FILE), wsl_binary).expect("bundle wsl");
}

fn write_installed_agent_bundle(install_dir: &Path, host_binary: &str, wsl_binary: &str) {
    fs::create_dir_all(install_dir).expect("install dir");
    fs::write(
        install_dir.join(AGENT_VERSION_FILE),
        r#"{"version":"1.0.0"}"#,
    )
    .expect("installed version");
    fs::write(install_dir.join(HOST_AGENT_BINARY_FILE), host_binary).expect("installed host");
    fs::write(install_dir.join(WSL_AGENT_BINARY_FILE), wsl_binary).expect("installed wsl");
}
