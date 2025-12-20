use crate::key_data::KeyData;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize)]
pub struct Config {
    public: PublicKey,
    secret: SecretKey,
}

impl Config {
    pub fn create(key_data: &KeyData) -> Result<Config> {
        let signing_key = key_data.signing_key();
        let userid = key_data.user_id().to_owned();
        let timestamp = key_data.timestamp();
        Ok(Config {
            public: PublicKey {
                key: hex::encode(signing_key.verifying_key().as_bytes()),
                userid,
                timestamp,
            },
            secret: SecretKey {
                key: Some(hex::encode(signing_key.to_bytes())),
                program: None,
            },
        })
    }

    pub fn load(file: &mut impl Read) -> Result<Config> {
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(toml::from_str(&buf)?)
    }

    pub fn write(&self, file: &mut impl Write) -> Result<()> {
        Ok(file.write_all(toml::to_string(self)?.as_bytes())?)
    }

    pub fn timestamp(&self) -> u64 {
        self.public.timestamp
    }

    pub fn user_id(&self) -> &str {
        &self.public.userid
    }

    pub fn public(&self) -> &str {
        &self.public.key
    }

    pub fn secret(&self) -> Result<Cow<'_, str>> {
        self.secret.secret()
    }
}

#[derive(Serialize, Deserialize)]
struct PublicKey {
    key: String,
    userid: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
struct SecretKey {
    key: Option<String>,
    program: Option<String>,
}

impl SecretKey {
    fn secret(&self) -> Result<Cow<'_, str>> {
        if let Some(key) = &self.key {
            Ok(Cow::Borrowed(key))
        } else if let Some(cmd) = &self.program {
            let mut args = cmd.split_whitespace();
            let cmd = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing command"))?;
            let output = std::process::Command::new(cmd).args(args).output()?;
            Ok(Cow::Owned(String::from_utf8(output.stdout)?))
        } else {
            bail!("No secret key or program specified")
        }
    }
}
