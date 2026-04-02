use anyhow::{anyhow, Result};
use fct_engine::ExecutionMode;

pub fn resolve_execution_mode(pure: bool, exec: bool) -> Result<(ExecutionMode, &'static str)> {
    if pure && exec {
        return Err(anyhow!("Use only one mode flag: --pure or --exec"));
    }

    if pure {
        Ok((ExecutionMode::Pure, "pure"))
    } else {
        Ok((ExecutionMode::Exec, "exec"))
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_execution_mode;
    use fct_engine::ExecutionMode;

    #[test]
    fn resolve_execution_mode_rejects_conflicting_flags() {
        assert!(resolve_execution_mode(true, true).is_err());
    }

    #[test]
    fn resolve_execution_mode_returns_pure_when_requested() {
        let (mode, label) = resolve_execution_mode(true, false).expect("pure mode");
        assert_eq!(mode, ExecutionMode::Pure);
        assert_eq!(label, "pure");
    }

    #[test]
    fn resolve_execution_mode_defaults_to_exec() {
        let (mode, label) = resolve_execution_mode(false, false).expect("default exec mode");
        assert_eq!(mode, ExecutionMode::Exec);
        assert_eq!(label, "exec");
    }
}
