mod v1;
mod v2;

pub use v1::LegacyConfig;
pub use v2::{Config, Identity, SecretKeySource};

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Detect config version and load appropriately
pub fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    // Try to detect version
    if content.contains("[identities.") || content.starts_with("version") {
        // V2 format
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse v2 config")?;
        Ok(config)
    } else if content.contains("[public]") && content.contains("[secret]") {
        // V1 format - convert to v2
        let legacy: LegacyConfig =
            toml::from_str(&content).with_context(|| "Failed to parse v1 config")?;
        Ok(legacy.into())
    } else {
        anyhow::bail!("Unrecognized config format")
    }
}

/// Save config to file
pub fn save_config(path: &Path, config: &Config) -> Result<()> {
    let content = toml::to_string_pretty(config).with_context(|| "Failed to serialize config")?;

    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create config file: {}", path.display()))?;

    file.write_all(content.as_bytes())
        .with_context(|| "Failed to write config file")?;

    Ok(())
}

/// Check if config file exists
pub fn config_exists(path: &Path) -> bool {
    path.exists()
}

/// Create a new config with a single identity
pub fn create_config(name: &str, identity: Identity) -> Config {
    let mut identities = HashMap::new();
    identities.insert(name.to_string(), identity);

    Config {
        version: 2,
        default: Some(name.to_string()),
        identities,
    }
}
