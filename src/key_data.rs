use std::time::SystemTime;

use anyhow::Result;
use ed25519_dalek::SigningKey;

use crate::config::Config;

pub struct KeyData {
    signing_key: SigningKey,
    user_id: String,
    timestamp: u64,
}

impl KeyData {
    pub fn create(signing_key: SigningKey, user_id: String, timestamp: u64) -> KeyData {
        KeyData {
            signing_key,
            user_id,
            timestamp,
        }
    }

    pub fn load(config: &Config) -> Result<KeyData> {
        let public_bytes: [u8; 32] = hex::decode(config.public())?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
        let secret_bytes: [u8; 32] = hex::decode(&*config.secret()?)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid secret key length"))?;

        let signing_key = SigningKey::from_bytes(&secret_bytes);

        // Verify the public key matches
        let expected_public = ed25519_dalek::VerifyingKey::from_bytes(&public_bytes)?;
        if signing_key.verifying_key() != expected_public {
            anyhow::bail!("Public key doesn't match secret key");
        }

        Ok(KeyData::create(
            signing_key,
            config.user_id().to_owned(),
            config.timestamp(),
        ))
    }

    pub fn sign(&self, data: &[u8]) -> Result<pbp::PgpSig> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        Ok(pbp::PgpSig::from_dalek::<sha2::Sha256, sha2::Sha512>(
            &self.signing_key,
            data,
            self.fingerprint(),
            pbp::SigType::BinaryDocument,
            timestamp as u32,
        ))
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn fingerprint(&self) -> pbp::Fingerprint {
        self.public().fingerprint()
    }

    pub fn public(&self) -> pbp::PgpKey {
        pbp::PgpKey::from_dalek::<sha2::Sha256, sha2::Sha512>(
            &self.signing_key,
            pbp::KeyFlags::SIGN | pbp::KeyFlags::CERTIFY,
            self.timestamp as u32,
            &self.user_id,
        )
    }
}
