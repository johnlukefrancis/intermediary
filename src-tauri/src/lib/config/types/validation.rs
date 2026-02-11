// Path: src-tauri/src/lib/config/types/validation.rs
// Description: Persisted configuration validation rules and invariants

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

use super::{
    PersistedConfig, CONFIG_VERSION, MAX_WINDOW_HEIGHT, MAX_WINDOW_WIDTH, MIN_WINDOW_HEIGHT,
    MIN_WINDOW_WIDTH,
};

/// Validate config invariants before saving
pub fn validate_config(config: &PersistedConfig) -> Result<(), String> {
    if config.config_version == 0 || config.config_version > CONFIG_VERSION {
        return Err(format!(
            "config_version {} is not supported (max {CONFIG_VERSION})",
            config.config_version
        ));
    }

    if config.agent_host.trim().is_empty() {
        return Err("agent_host must not be empty".to_string());
    }

    if config.agent_port < 1024 {
        return Err("agent_port must be >= 1024".to_string());
    }

    if let Some(distro) = &config.agent_distro {
        if distro.trim().is_empty() {
            return Err("agent_distro must not be empty when provided".to_string());
        }
    }

    if config.recent_files_limit < 25 || config.recent_files_limit > 2000 {
        return Err(format!(
            "recent_files_limit must be 25-2000, got {}",
            config.recent_files_limit
        ));
    }

    if config.window_opacity_percent > 100 {
        return Err(format!(
            "window_opacity_percent must be 0-100, got {}",
            config.window_opacity_percent
        ));
    }

    if config.texture_intensity_percent > 100 {
        return Err(format!(
            "texture_intensity_percent must be 0-100, got {}",
            config.texture_intensity_percent
        ));
    }

    let mut repo_ids = HashSet::new();
    for repo in &config.repos {
        validate_non_empty(&repo.repo_id, "repo.repo_id")?;
        validate_non_empty(&repo.label, "repo.label")?;
        validate_non_empty(repo.root.path(), "repo.root.path")?;

        if !repo_ids.insert(repo.repo_id.as_str()) {
            return Err(format!("duplicate repo_id: {}", repo.repo_id));
        }

        for preset in &repo.bundle_presets {
            validate_non_empty(&preset.preset_id, "bundle_preset.preset_id")?;
            validate_non_empty(&preset.preset_name, "bundle_preset.preset_name")?;
        }
    }

    for (tab_key, theme) in &config.tab_themes {
        validate_accent_hex(&theme.accent_hex, tab_key)?;
        if let Some(texture_id) = &theme.texture_id {
            if texture_id.trim().is_empty() {
                return Err(format!(
                    "tabTheme texture_id for {tab_key} must not be empty when provided"
                ));
            }
        }
    }

    for (mode_key, bounds) in &config.ui_state.window_bounds_by_mode {
        if mode_key != "standard" && mode_key != "handset" {
            return Err(format!(
                "ui_state.window_bounds_by_mode has invalid mode key: {mode_key}"
            ));
        }
        if bounds.width < MIN_WINDOW_WIDTH || bounds.width > MAX_WINDOW_WIDTH {
            return Err(format!(
                "ui_state.window_bounds_by_mode[{mode_key}].width must be {MIN_WINDOW_WIDTH}-{MAX_WINDOW_WIDTH}, got {}",
                bounds.width
            ));
        }
        if bounds.height < MIN_WINDOW_HEIGHT || bounds.height > MAX_WINDOW_HEIGHT {
            return Err(format!(
                "ui_state.window_bounds_by_mode[{mode_key}].height must be {MIN_WINDOW_HEIGHT}-{MAX_WINDOW_HEIGHT}, got {}",
                bounds.height
            ));
        }
    }

    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

/// Regex for #RRGGBB hex color format
static ACCENT_HEX_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").expect("valid regex"));

fn validate_accent_hex(value: &str, tab_key: &str) -> Result<(), String> {
    if !ACCENT_HEX_REGEX.is_match(value) {
        return Err(format!(
            "tabTheme accent_hex for {tab_key} must be #RRGGBB format, got: {value}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_config;
    use crate::config::types::PersistedConfig;

    #[test]
    fn accepts_window_opacity_in_range() {
        let mut config = PersistedConfig::default();
        config.window_opacity_percent = 0;
        assert!(validate_config(&config).is_ok());
        config.window_opacity_percent = 100;
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn rejects_window_opacity_above_100() {
        let mut config = PersistedConfig::default();
        config.window_opacity_percent = 101;
        let error = validate_config(&config).expect_err("expected validation error");
        assert!(error.contains("window_opacity_percent"));
    }

    #[test]
    fn accepts_texture_intensity_in_range() {
        let mut config = PersistedConfig::default();
        config.texture_intensity_percent = 0;
        assert!(validate_config(&config).is_ok());
        config.texture_intensity_percent = 100;
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn rejects_texture_intensity_above_100() {
        let mut config = PersistedConfig::default();
        config.texture_intensity_percent = 101;
        let error = validate_config(&config).expect_err("expected validation error");
        assert!(error.contains("texture_intensity_percent"));
    }
}
