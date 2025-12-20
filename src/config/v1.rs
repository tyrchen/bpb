//! Legacy v1 configuration format support
//!
//! This module handles the original single-identity config format:
//! ```toml
//! [public]
//! key = "..."
//! userid = "..."
//! timestamp = 1234567890
//!
//! [secret]
//! key = "..."
//! # or: program = "..."
//! ```

use serde::{Deserialize, Serialize};

use super::{Config, Identity, SecretKeySource};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct LegacyConfig {
    pub public: LegacyPublicKey,
    pub secret: LegacySecretKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LegacyPublicKey {
    pub key: String,
    pub userid: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LegacySecretKey {
    pub key: Option<String>,
    pub program: Option<String>,
}

impl From<LegacyConfig> for Config {
    fn from(legacy: LegacyConfig) -> Self {
        let secret = if let Some(key) = legacy.secret.key {
            SecretKeySource::inline(key)
        } else if let Some(program) = legacy.secret.program {
            SecretKeySource::program(program)
        } else {
            // This shouldn't happen with valid configs, but handle it gracefully
            SecretKeySource::inline(String::new())
        };

        let identity = Identity {
            userid: legacy.public.userid,
            timestamp: legacy.public.timestamp,
            public_key: legacy.public.key,
            secret,
        };

        let mut identities = HashMap::new();
        identities.insert("default".to_string(), identity);

        Config {
            version: 2,
            default: Some("default".to_string()),
            identities,
        }
    }
}

impl From<&Config> for Option<LegacyConfig> {
    /// Convert back to legacy format if there's only one identity named "default"
    fn from(config: &Config) -> Self {
        use super::v2::{InlineSecret, ProgramSecret};

        if config.identities.len() == 1
            && let Some(identity) = config.identities.get("default")
        {
            let secret = match &identity.secret {
                SecretKeySource::Inline(InlineSecret { key }) => LegacySecretKey {
                    key: Some(key.clone()),
                    program: None,
                },
                SecretKeySource::Program(ProgramSecret { program }) => LegacySecretKey {
                    key: None,
                    program: Some(program.clone()),
                },
            };

            return Some(LegacyConfig {
                public: LegacyPublicKey {
                    key: identity.public_key.clone(),
                    userid: identity.userid.clone(),
                    timestamp: identity.timestamp,
                },
                secret,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_config() {
        let toml = r#"
[public]
key = "abc123"
userid = "Test User <test@example.com>"
timestamp = 1234567890

[secret]
key = "secret123"
"#;
        let legacy: LegacyConfig = toml::from_str(toml).unwrap();
        assert_eq!(legacy.public.userid, "Test User <test@example.com>");
        assert_eq!(legacy.secret.key, Some("secret123".to_string()));
    }

    #[test]
    fn test_legacy_to_v2_conversion() {
        let legacy = LegacyConfig {
            public: LegacyPublicKey {
                key: "pubkey".to_string(),
                userid: "User <user@test.com>".to_string(),
                timestamp: 1234567890,
            },
            secret: LegacySecretKey {
                key: Some("secretkey".to_string()),
                program: None,
            },
        };

        let config: Config = legacy.into();
        assert_eq!(config.version, 2);
        assert_eq!(config.default, Some("default".to_string()));
        assert!(config.identities.contains_key("default"));

        let identity = config.identities.get("default").unwrap();
        assert_eq!(identity.userid, "User <user@test.com>");
        assert_eq!(identity.public_key, "pubkey");
    }
}
