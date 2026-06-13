//! MCP daemon autostart policy (consent + user setting).

use dayrecord_core::control::ControlError;
use dayrecord_core::ports::Repository;

pub const SETTING_MCP_AUTOSTART: &str = "mcp_autostart_daemon";

/// Returns `Ok(())` when MCP may spawn `dayrecord daemon` on control-tool calls.
pub fn mcp_autostart_allowed<R: Repository>(repo: &R) -> Result<(), ControlError> {
    if std::env::var_os("DAYRECORD_MCP_DISABLE_AUTOSTART").is_some() {
        return Err(ControlError::AutostartDenied(
            "MCP daemon autostart disabled by DAYRECORD_MCP_DISABLE_AUTOSTART".into(),
        ));
    }

    let consent = repo
        .get_setting("consent")
        .map_err(|e| ControlError::Transport(e.to_string()))?;
    if consent.as_deref() != Some("true") {
        return Err(ControlError::AutostartDenied(
            "Data collection consent is not granted. Run `dayrecord consent --accept true` \
             or accept in the DayRecord GUI before MCP can start the capture service."
                .into(),
        ));
    }

    let autostart = repo
        .get_setting(SETTING_MCP_AUTOSTART)
        .map_err(|e| ControlError::Transport(e.to_string()))?;
    if autostart.as_deref() == Some("false") {
        return Err(ControlError::AutostartDenied(
            "MCP daemon autostart is disabled (`mcp_autostart_daemon=false`). \
             Start DayRecord GUI or run `dayrecord daemon` manually."
                .into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dayrecord_core::ports::InMemoryRepository;

    #[test]
    fn denied_without_consent() {
        let repo = InMemoryRepository::default();
        let err = mcp_autostart_allowed(&repo).unwrap_err();
        assert!(matches!(err, ControlError::AutostartDenied(_)));
    }

    #[test]
    fn allowed_with_consent_default_autostart() {
        let repo = InMemoryRepository::default();
        repo.set_setting("consent", "true").unwrap();
        assert!(mcp_autostart_allowed(&repo).is_ok());
    }

    #[test]
    fn denied_when_autostart_disabled() {
        let repo = InMemoryRepository::default();
        repo.set_setting("consent", "true").unwrap();
        repo.set_setting(SETTING_MCP_AUTOSTART, "false").unwrap();
        let err = mcp_autostart_allowed(&repo).unwrap_err();
        assert!(matches!(err, ControlError::AutostartDenied(_)));
    }
}
