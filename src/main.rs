mod cli;
mod config;
mod git;
mod key_data;

use crate::cli::{Args, Command};
use crate::config::{config_exists, create_config, load_config, save_config};
use crate::key_data::KeyData;
use anyhow::{Context, Result, bail};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::io::Read;
use std::path::PathBuf;
use std::time::SystemTime;

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init { userid, identity } => cmd_init(userid, identity),
        Command::List => cmd_list(),
        Command::Print { identity } => cmd_print(identity),
        Command::Remove { identity } => cmd_remove(identity),
        Command::Rename { old_name, new_name } => cmd_rename(old_name, new_name),
        Command::Default { identity } => cmd_default(identity),
        Command::Export {
            identity,
            include_secret,
        } => cmd_export(identity, include_secret),
        Command::Sign { identity } => cmd_sign(identity),
        Command::Help => cmd_help(),
        Command::Delegate => cmd_delegate(),
    }
}

/// Get the config file path
fn config_path() -> PathBuf {
    std::env::var("BPB_KEYS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".bpb_keys.toml")
        })
}

/// Initialize a new identity
fn cmd_init(userid: String, identity_name: Option<String>) -> Result<()> {
    let path = config_path();
    let identity_name = identity_name.unwrap_or_else(|| "default".to_string());

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let signing_key = SigningKey::generate(&mut OsRng);
    let key_data = KeyData::create(signing_key, userid, timestamp);
    let identity = key_data.to_identity();

    if config_exists(&path) {
        // Add to existing config
        let mut config = load_config(&path)?;

        if config.has_identity(&identity_name) {
            bail!(
                "Identity '{}' already exists. Use a different name with --id <name>, \
                or remove the existing identity first with `bpb remove {}`",
                identity_name,
                identity_name
            );
        }

        config.set_identity(identity_name.clone(), identity);
        save_config(&path, &config)?;
        eprintln!("Added identity '{}' to config", identity_name);
    } else {
        // Create new config
        let config = create_config(&identity_name, identity);
        save_config(&path, &config)?;
        eprintln!("Created new config with identity '{}'", identity_name);
    }

    println!("{}", key_data.public());
    Ok(())
}

/// List all identities
fn cmd_list() -> Result<()> {
    let path = config_path();
    let config =
        load_config(&path).with_context(|| "No config found. Run `bpb init <userid>` first.")?;

    let default_name = config.default_identity_name();

    if config.identity_count() == 0 {
        println!("No identities configured.");
        return Ok(());
    }

    println!("Identities:");
    let mut names: Vec<_> = config.list_identity_names();
    names.sort();

    for name in names {
        let marker = if Some(name) == default_name {
            " (default)"
        } else {
            ""
        };
        if let Some(identity) = config.get_identity(name) {
            println!("  {} - {}{}", name, identity.userid, marker);
        }
    }

    Ok(())
}

/// Print public key for an identity
fn cmd_print(identity_name: Option<String>) -> Result<()> {
    let path = config_path();
    let config =
        load_config(&path).with_context(|| "No config found. Run `bpb init <userid>` first.")?;

    // Resolve identity: explicit > env > git config > default
    let resolved = git::resolve_identity(identity_name.as_deref());
    let name = resolved
        .as_deref()
        .or_else(|| config.default_identity_name())
        .ok_or_else(|| anyhow::anyhow!("No identity specified and no default set"))?;

    let identity = config
        .get_identity(name)
        .ok_or_else(|| anyhow::anyhow!("Identity '{}' not found", name))?;

    let key_data = KeyData::from_identity(identity)?;
    println!("{}", key_data.public());
    Ok(())
}

/// Remove an identity
fn cmd_remove(identity_name: String) -> Result<()> {
    let path = config_path();
    let mut config = load_config(&path)?;

    if !config.has_identity(&identity_name) {
        bail!("Identity '{}' not found", identity_name);
    }

    if config.identity_count() == 1 {
        bail!(
            "Cannot remove the last identity. Delete the config file at {} instead.",
            path.display()
        );
    }

    config.remove_identity(&identity_name);
    save_config(&path, &config)?;

    eprintln!("Removed identity '{}'", identity_name);
    if let Some(new_default) = config.default_identity_name() {
        eprintln!("Default is now '{}'", new_default);
    }

    Ok(())
}

/// Rename an identity
fn cmd_rename(old_name: String, new_name: String) -> Result<()> {
    let path = config_path();
    let mut config = load_config(&path)?;

    config.rename_identity(&old_name, new_name.clone())?;
    save_config(&path, &config)?;

    eprintln!("Renamed '{}' to '{}'", old_name, new_name);
    Ok(())
}

/// Set the default identity
fn cmd_default(identity_name: String) -> Result<()> {
    let path = config_path();
    let mut config = load_config(&path)?;

    config.set_default(&identity_name)?;
    save_config(&path, &config)?;

    eprintln!("Default identity set to '{}'", identity_name);
    Ok(())
}

/// Export an identity
fn cmd_export(identity_name: String, include_secret: bool) -> Result<()> {
    let path = config_path();
    let config = load_config(&path)?;

    let identity = config
        .get_identity(&identity_name)
        .ok_or_else(|| anyhow::anyhow!("Identity '{}' not found", identity_name))?;

    if include_secret {
        // Export full identity including secret
        let export = config::Config {
            version: 2,
            default: Some(identity_name.clone()),
            identities: [(identity_name, identity.clone())].into_iter().collect(),
        };
        println!("{}", toml::to_string_pretty(&export)?);
    } else {
        // Export public key only
        let key_data = KeyData::from_identity(identity)?;
        println!("{}", key_data.public());
    }

    Ok(())
}

/// Sign data from stdin (for git integration)
fn cmd_sign(identity_name: Option<String>) -> Result<()> {
    let path = config_path();
    let config =
        load_config(&path).with_context(|| "No config found. Run `bpb init <userid>` first.")?;

    // Resolve identity: explicit > env > git config > default
    let resolved = git::resolve_identity(identity_name.as_deref());
    let name = resolved
        .as_deref()
        .or_else(|| config.default_identity_name())
        .ok_or_else(|| anyhow::anyhow!("No identity specified and no default set"))?;

    let identity = config
        .get_identity(name)
        .ok_or_else(|| anyhow::anyhow!("Identity '{}' not found", name))?;

    let key_data = KeyData::from_identity(identity)?;

    let mut data = String::new();
    std::io::stdin().read_to_string(&mut data)?;

    let sig = key_data.sign(data.as_bytes())?;

    // Output in GPG-compatible format for git
    eprintln!("\n[GNUPG:] SIG_CREATED ");
    println!("{}", sig);

    Ok(())
}

/// Print help message
fn cmd_help() -> Result<()> {
    println!("bpb: boats's personal barricade");
    println!();
    println!("A tool for signing git commits with ed25519 keys.");
    println!();
    println!("USAGE:");
    println!("    bpb <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    init <userid>           Create a new identity");
    println!("        --id, -i <name>     Name for the identity (default: \"default\")");
    println!();
    println!("    list, ls                List all identities");
    println!();
    println!("    print                   Print public key in OpenPGP format");
    println!("        --id, -i <name>     Identity to print (uses default if not specified)");
    println!();
    println!("    remove, rm <name>       Remove an identity");
    println!();
    println!("    rename, mv <old> <new>  Rename an identity");
    println!();
    println!("    default <name>          Set the default identity");
    println!();
    println!("    export <name>           Export an identity");
    println!("        --secret, -s        Include secret key in export");
    println!();
    println!("    --help, -h, help        Show this help message");
    println!();
    println!("IDENTITY SELECTION:");
    println!("    Identity is resolved in this order:");
    println!("    1. Explicit --id flag");
    println!("    2. BPB_IDENTITY environment variable");
    println!("    3. Repository-local git config: bpb.identity");
    println!("    4. Global git config: bpb.identity");
    println!("    5. Default identity from config");
    println!();
    println!("GIT INTEGRATION:");
    println!("    Configure git to use bpb for signing:");
    println!("        git config --global gpg.program bpb");
    println!("        git config --global commit.gpgsign true");
    println!();
    println!("    Set per-repository identity:");
    println!("        git config bpb.identity work");
    println!();
    println!("See https://github.com/withoutboats/bpb for more information.");
    Ok(())
}

/// Delegate to real GPG
fn cmd_delegate() -> ! {
    use std::process;

    let mut cmd = process::Command::new("gpg");
    cmd.args(std::env::args().skip(1));
    let status = cmd.status().unwrap().code().unwrap();
    process::exit(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_path_default() {
        // Just verify it doesn't panic
        let _ = config_path();
    }
}
