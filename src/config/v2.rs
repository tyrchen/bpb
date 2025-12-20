//! V2 configuration format with multiple identity support
//!
//! ```toml
//! version = 2
//! default = "personal"
//!
//! [identities.personal]
//! userid = "User <user@personal.dev>"
//! timestamp = 1234567890
//! public_key = "..."
//!
//! [identities.personal.secret]
//! key = "..."
//!
//! [identities.work]
//! userid = "User <user@work.com>"
//! timestamp = 1234567891
//! public_key = "..."
//!
//! [identities.work.secret]
//! program = "pass show bpb/work"
//! ```

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default)]
    pub identities: HashMap<String, Identity>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    pub userid: String,
    pub timestamp: u64,
    pub public_key: String,
    pub secret: SecretKeySource,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SecretKeySource {
    Inline(InlineSecret),
    Program(ProgramSecret),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InlineSecret {
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramSecret {
    pub program: String,
}

impl SecretKeySource {
    pub fn inline(key: String) -> Self {
        SecretKeySource::Inline(InlineSecret { key })
    }

    pub fn program(program: String) -> Self {
        SecretKeySource::Program(ProgramSecret { program })
    }

    pub fn get_secret(&self) -> Result<Cow<'_, str>> {
        match self {
            SecretKeySource::Inline(s) => Ok(Cow::Borrowed(&s.key)),
            SecretKeySource::Program(p) => {
                let mut args = p.program.split_whitespace();
                let cmd = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Missing command in program"))?;
                let output = std::process::Command::new(cmd).args(args).output()?;
                if !output.status.success() {
                    bail!(
                        "Secret program failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Ok(Cow::Owned(
                    String::from_utf8(output.stdout)?.trim().to_string(),
                ))
            }
        }
    }
}

impl Config {
    /// Create a new empty config
    pub fn new() -> Self {
        Config {
            version: 2,
            default: None,
            identities: HashMap::new(),
        }
    }

    /// Get the default identity name
    pub fn default_identity_name(&self) -> Option<&str> {
        self.default.as_deref().or_else(|| {
            // If no default set, return the first (and possibly only) identity
            self.identities.keys().next().map(|s| s.as_str())
        })
    }

    /// Get the default identity
    pub fn default_identity(&self) -> Option<&Identity> {
        self.default_identity_name()
            .and_then(|name| self.identities.get(name))
    }

    /// Get an identity by name
    pub fn get_identity(&self, name: &str) -> Option<&Identity> {
        self.identities.get(name)
    }

    /// Add or update an identity
    pub fn set_identity(&mut self, name: String, identity: Identity) {
        // If this is the first identity, make it the default
        if self.identities.is_empty() {
            self.default = Some(name.clone());
        }
        self.identities.insert(name, identity);
    }

    /// Remove an identity
    pub fn remove_identity(&mut self, name: &str) -> Option<Identity> {
        let removed = self.identities.remove(name);

        // If we removed the default, clear it or set to another identity
        if self.default.as_deref() == Some(name) {
            self.default = self.identities.keys().next().cloned();
        }

        removed
    }

    /// Rename an identity
    pub fn rename_identity(&mut self, old_name: &str, new_name: String) -> Result<()> {
        if !self.identities.contains_key(old_name) {
            bail!("Identity '{}' not found", old_name);
        }
        if self.identities.contains_key(&new_name) {
            bail!("Identity '{}' already exists", new_name);
        }

        if let Some(identity) = self.identities.remove(old_name) {
            self.identities.insert(new_name.clone(), identity);

            // Update default if needed
            if self.default.as_deref() == Some(old_name) {
                self.default = Some(new_name);
            }
        }

        Ok(())
    }

    /// Set the default identity
    pub fn set_default(&mut self, name: &str) -> Result<()> {
        if !self.identities.contains_key(name) {
            bail!("Identity '{}' not found", name);
        }
        self.default = Some(name.to_string());
        Ok(())
    }

    /// List all identity names
    pub fn list_identity_names(&self) -> Vec<&str> {
        self.identities.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an identity exists
    pub fn has_identity(&self, name: &str) -> bool {
        self.identities.contains_key(name)
    }

    /// Get the number of identities
    pub fn identity_count(&self) -> usize {
        self.identities.len()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v2_config() {
        let toml = r#"
version = 2
default = "personal"

[identities.personal]
userid = "User <user@personal.dev>"
timestamp = 1234567890
public_key = "pubkey123"

[identities.personal.secret]
key = "secretkey123"

[identities.work]
userid = "User <user@work.com>"
timestamp = 1234567891
public_key = "workpubkey"

[identities.work.secret]
program = "pass show bpb/work"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.version, 2);
        assert_eq!(config.default, Some("personal".to_string()));
        assert_eq!(config.identities.len(), 2);

        let personal = config.identities.get("personal").unwrap();
        assert_eq!(personal.userid, "User <user@personal.dev>");

        let work = config.identities.get("work").unwrap();
        assert_eq!(work.userid, "User <user@work.com>");
    }

    #[test]
    fn test_config_operations() {
        let mut config = Config::new();

        let identity = Identity {
            userid: "Test <test@test.com>".to_string(),
            timestamp: 1234567890,
            public_key: "pubkey".to_string(),
            secret: SecretKeySource::inline("secret".to_string()),
        };

        config.set_identity("test".to_string(), identity);
        assert!(config.has_identity("test"));
        assert_eq!(config.default_identity_name(), Some("test"));

        config
            .rename_identity("test", "renamed".to_string())
            .unwrap();
        assert!(!config.has_identity("test"));
        assert!(config.has_identity("renamed"));
        assert_eq!(config.default_identity_name(), Some("renamed"));

        config.remove_identity("renamed");
        assert!(!config.has_identity("renamed"));
        assert_eq!(config.identity_count(), 0);
    }

    #[test]
    fn test_serialize_config() {
        let mut config = Config::new();
        config.set_identity(
            "test".to_string(),
            Identity {
                userid: "Test <test@test.com>".to_string(),
                timestamp: 1234567890,
                public_key: "pubkey".to_string(),
                secret: SecretKeySource::inline("secret".to_string()),
            },
        );

        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("version = 2"));
        assert!(serialized.contains("[identities.test]"));
    }
}
