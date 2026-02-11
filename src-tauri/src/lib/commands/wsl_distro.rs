// Path: src-tauri/src/lib/commands/wsl_distro.rs
// Description: Resolve WSL distro override from persisted app config for command-time path conversion

use crate::config::{load_from_disk, resolve_config_path};
use std::sync::RwLock;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct WslDistroState {
    value: RwLock<Option<String>>,
}

impl WslDistroState {
    pub fn set(&self, distro: Option<&str>) {
        if let Ok(mut guard) = self.value.write() {
            *guard = normalize_distro(distro);
        }
    }

    pub fn get(&self) -> Option<String> {
        self.value.read().ok().and_then(|guard| guard.clone())
    }
}

pub(crate) fn resolve_runtime_wsl_distro(
    app: &AppHandle,
    distro_override: Option<&str>,
) -> Option<String> {
    if let Some(override_value) = normalize_distro(distro_override) {
        return Some(override_value);
    }

    if let Some(state) = app.try_state::<WslDistroState>() {
        if let Some(state_value) = state.get() {
            return Some(state_value);
        }
    }

    resolve_configured_wsl_distro(app)
}

fn resolve_configured_wsl_distro(app: &AppHandle) -> Option<String> {
    let config_path = resolve_config_path(app).ok()?;
    let loaded = load_from_disk(&config_path).ok()?;
    normalize_distro(loaded.config.agent_distro.as_deref())
}

fn normalize_distro(distro: Option<&str>) -> Option<String> {
    distro
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::normalize_distro;

    #[test]
    fn normalize_distro_trims_and_filters_empty() {
        assert_eq!(normalize_distro(None), None);
        assert_eq!(normalize_distro(Some("   ")), None);
        assert_eq!(
            normalize_distro(Some("  Ubuntu-22.04  ")),
            Some("Ubuntu-22.04".to_string())
        );
    }
}
