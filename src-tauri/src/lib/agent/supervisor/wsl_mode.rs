// Path: src-tauri/src/lib/agent/supervisor/wsl_mode.rs
// Description: WSL backend mode parsing and ownership-policy helpers for the supervisor

const WSL_BACKEND_MODE_ENV: &str = "INTERMEDIARY_WSL_BACKEND_MODE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WslBackendMode {
    Auto,
    Managed,
    External,
}

impl WslBackendMode {
    pub(super) fn log_key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Managed => "managed",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WslBackendOwner {
    InstalledManaged,
    ExternalUnmanaged,
}

impl WslBackendOwner {
    pub(super) fn log_key(self) -> &'static str {
        match self {
            Self::InstalledManaged => "installed_managed",
            Self::ExternalUnmanaged => "external_unmanaged",
        }
    }
}

pub(super) fn resolve_wsl_backend_mode() -> (WslBackendMode, Option<String>) {
    let raw = std::env::var(WSL_BACKEND_MODE_ENV).ok();
    parse_wsl_backend_mode(raw.as_deref())
}

pub(super) fn wsl_backend_mode_requires_managed_owner() -> bool {
    matches!(resolve_wsl_backend_mode().0, WslBackendMode::Managed)
}

pub(super) fn backend_mode_allows_owner(mode: WslBackendMode, owner: WslBackendOwner) -> bool {
    !matches!(
        (mode, owner),
        (WslBackendMode::Managed, WslBackendOwner::ExternalUnmanaged)
    )
}

fn parse_wsl_backend_mode(raw: Option<&str>) -> (WslBackendMode, Option<String>) {
    let Some(raw_mode) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return (WslBackendMode::Auto, None);
    };

    if raw_mode.eq_ignore_ascii_case("auto") {
        return (WslBackendMode::Auto, None);
    }
    if raw_mode.eq_ignore_ascii_case("managed") {
        return (WslBackendMode::Managed, None);
    }
    if raw_mode.eq_ignore_ascii_case("external") {
        return (WslBackendMode::External, None);
    }

    (WslBackendMode::Auto, Some(raw_mode.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        backend_mode_allows_owner, parse_wsl_backend_mode, WslBackendMode, WslBackendOwner,
    };

    #[test]
    fn parse_wsl_backend_mode_defaults_to_auto_when_missing_or_empty() {
        let (mode_missing, invalid_missing) = parse_wsl_backend_mode(None);
        assert_eq!(mode_missing, WslBackendMode::Auto);
        assert_eq!(invalid_missing, None);

        let (mode_empty, invalid_empty) = parse_wsl_backend_mode(Some("  "));
        assert_eq!(mode_empty, WslBackendMode::Auto);
        assert_eq!(invalid_empty, None);
    }

    #[test]
    fn parse_wsl_backend_mode_recognizes_valid_values() {
        assert_eq!(parse_wsl_backend_mode(Some("auto")).0, WslBackendMode::Auto);
        assert_eq!(
            parse_wsl_backend_mode(Some("managed")).0,
            WslBackendMode::Managed
        );
        assert_eq!(
            parse_wsl_backend_mode(Some("external")).0,
            WslBackendMode::External
        );
    }

    #[test]
    fn parse_wsl_backend_mode_returns_invalid_raw_value() {
        let (mode, invalid) = parse_wsl_backend_mode(Some("weird"));
        assert_eq!(mode, WslBackendMode::Auto);
        assert_eq!(invalid.as_deref(), Some("weird"));
    }

    #[test]
    fn managed_mode_rejects_external_owner() {
        assert!(!backend_mode_allows_owner(
            WslBackendMode::Managed,
            WslBackendOwner::ExternalUnmanaged
        ));
    }
}
