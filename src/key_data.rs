use std::time::SystemTime;

use anyhow::Result;
use ed25519_dalek::SigningKey;

use crate::config::Identity;

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

    pub fn from_identity(identity: &Identity) -> Result<KeyData> {
        let public_bytes: [u8; 32] = hex::decode(&identity.public_key)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;

        let secret_hex = identity.secret.get_secret()?;
        let secret_bytes: [u8; 32] = hex::decode(secret_hex.trim())?
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
            identity.userid.clone(),
            identity.timestamp,
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

    #[allow(dead_code)]
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    #[allow(dead_code)]
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    #[allow(dead_code)]
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

    /// Create an Identity from this KeyData
    pub fn to_identity(&self) -> Identity {
        use crate::config::SecretKeySource;

        Identity {
            userid: self.user_id.clone(),
            timestamp: self.timestamp,
            public_key: hex::encode(self.signing_key.verifying_key().as_bytes()),
            secret: SecretKeySource::inline(hex::encode(self.signing_key.to_bytes())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecretKeySource;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_key_data_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let key_data = KeyData::create(signing_key, "Test <test@test.com>".to_string(), 1234567890);

        let identity = key_data.to_identity();
        let loaded = KeyData::from_identity(&identity).unwrap();

        assert_eq!(loaded.user_id(), key_data.user_id());
        assert_eq!(loaded.timestamp(), key_data.timestamp());
        assert_eq!(loaded.fingerprint(), key_data.fingerprint());
    }

    #[test]
    fn test_from_identity() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let identity = Identity {
            userid: "Test User <test@example.com>".to_string(),
            timestamp: 1234567890,
            public_key: hex::encode(signing_key.verifying_key().as_bytes()),
            secret: SecretKeySource::inline(hex::encode(signing_key.to_bytes())),
        };

        let key_data = KeyData::from_identity(&identity).unwrap();
        assert_eq!(key_data.user_id(), "Test User <test@example.com>");
    }
}
