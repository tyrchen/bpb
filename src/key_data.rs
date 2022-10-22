use std::time::SystemTime;

use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use failure::Error;

use crate::config::Config;

pub struct KeyData {
    keypair: Keypair,
    user_id: String,
    timestamp: u64,
}

impl KeyData {
    pub fn create(keypair: Keypair, user_id: String, timestamp: u64) -> KeyData {
        KeyData {
            keypair,
            user_id,
            timestamp,
        }
    }

    pub fn load(config: &Config) -> Result<KeyData, Error> {
        let keypair = Keypair {
            public: PublicKey::from_bytes(&hex::decode(config.public())?)?,
            secret: SecretKey::from_bytes(&hex::decode(&*config.secret()?)?)?,
        };
        Ok(KeyData::create(
            keypair,
            config.user_id().to_owned(),
            config.timestamp(),
        ))
    }

    pub fn sign(&self, data: &[u8]) -> Result<pbp::PgpSig, Error> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        Ok(pbp::PgpSig::from_dalek::<sha2::Sha256, sha2::Sha512>(
            &self.keypair,
            data,
            self.fingerprint(),
            pbp::SigType::BinaryDocument,
            timestamp as u32,
        ))
    }

    pub fn keypair(&self) -> &Keypair {
        &self.keypair
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
            &self.keypair,
            pbp::KeyFlags::SIGN | pbp::KeyFlags::CERTIFY,
            self.timestamp as u32,
            &self.user_id,
        )
    }
}
