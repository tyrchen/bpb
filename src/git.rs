//! Git integration for identity selection

use std::env;
use std::process::Command;

/// Get the identity to use, checking various sources in order:
/// 1. Explicit identity parameter
/// 2. BPB_IDENTITY environment variable
/// 3. Repository-local git config (bpb.identity)
/// 4. Global git config (bpb.identity)
/// 5. None (use default from config)
pub fn resolve_identity(explicit: Option<&str>) -> Option<String> {
    // 1. Explicit parameter takes priority
    if let Some(id) = explicit {
        return Some(id.to_string());
    }

    // 2. Environment variable
    if let Ok(id) = env::var("BPB_IDENTITY")
        && !id.is_empty()
    {
        return Some(id);
    }

    // 3. Repository-local git config
    if let Some(id) = get_git_config("bpb.identity", false) {
        return Some(id);
    }

    // 4. Global git config
    if let Some(id) = get_git_config("bpb.identity", true) {
        return Some(id);
    }

    // 5. None - caller should use default from config
    None
}

/// Get a git config value
fn get_git_config(key: &str, global: bool) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("config");

    if global {
        cmd.arg("--global");
    }

    cmd.arg("--get").arg(key);

    let output = cmd.output().ok()?;

    if output.status.success() {
        let value = String::from_utf8(output.stdout).ok()?;
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Check if we're inside a git repository
#[allow(dead_code)]
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_identity_explicit() {
        let result = resolve_identity(Some("work"));
        assert_eq!(result, Some("work".to_string()));
    }

    #[test]
    fn test_resolve_identity_env() {
        // SAFETY: Tests run serially and we restore the env var immediately
        unsafe {
            env::set_var("BPB_IDENTITY", "env_identity");
        }
        let result = resolve_identity(None);
        assert_eq!(result, Some("env_identity".to_string()));
        unsafe {
            env::remove_var("BPB_IDENTITY");
        }
    }

    #[test]
    fn test_resolve_identity_explicit_overrides_env() {
        // SAFETY: Tests run serially and we restore the env var immediately
        unsafe {
            env::set_var("BPB_IDENTITY", "env_identity");
        }
        let result = resolve_identity(Some("explicit"));
        assert_eq!(result, Some("explicit".to_string()));
        unsafe {
            env::remove_var("BPB_IDENTITY");
        }
    }
}
