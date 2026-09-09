use std::path::{Path, PathBuf};
use anyhow::Context;
use mogh_pki::{EncodedKeyPair, PkiKind, SpkiPublicKey};
use tracing::info;

#[derive(Clone)]
pub struct IdentityKeys {
    pub private_key: String,
    pub public_key: SpkiPublicKey,
}

impl IdentityKeys {
    /// Load existing persistent identity keys from disk or generate a new keypair.
    pub fn load_or_generate(keys_dir: &Path) -> anyhow::Result<Self> {
        let key_file = keys_dir.join("periphery.key");
        std::fs::create_dir_all(keys_dir)
            .with_context(|| format!("Failed to create keys directory at {}", keys_dir.display()))?;

        let keypair = EncodedKeyPair::load_maybe_generate(PkiKind::Mutual, &key_file)
            .with_context(|| format!("Failed to load or generate keypair at {}", key_file.display()))?;

        info!("Loaded identity keys from {}", key_file.display());
        Ok(Self {
            private_key: keypair.private.as_str().to_string(),
            public_key: keypair.public,
        })
    }

    /// Generate an ephemeral keypair (useful for tests or onboarding tokens).
    pub fn generate() -> anyhow::Result<Self> {
        let keypair = EncodedKeyPair::generate(PkiKind::Mutual)?;
        Ok(Self {
            private_key: keypair.private.as_str().to_string(),
            public_key: keypair.public,
        })
    }

    /// Load from a raw private key string (PEM or base64).
    pub fn from_private_key_str(private_key: &str) -> anyhow::Result<Self> {
        let keypair = EncodedKeyPair::from_private_key(PkiKind::Mutual, private_key)?;
        Ok(Self {
            private_key: keypair.private.as_str().to_string(),
            public_key: keypair.public,
        })
    }

    /// Default keys directory path on Android under Magisk.
    pub fn default_path() -> PathBuf {
        if Path::new("/data/adb").exists() {
            PathBuf::from("/data/adb/komodo/keys")
        } else {
            PathBuf::from("./keys")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_keys_generate_and_reload() {
        let temp_dir = std::env::temp_dir().join(format!("komodo_test_keys_{}", uuid::Uuid::new_v4()));
        let keys1 = IdentityKeys::load_or_generate(&temp_dir).unwrap();
        assert!(!keys1.private_key.is_empty());
        assert!(!keys1.public_key.as_str().is_empty());

        let keys2 = IdentityKeys::load_or_generate(&temp_dir).unwrap();
        assert_eq!(keys1.private_key, keys2.private_key);
        assert_eq!(keys1.public_key.as_str(), keys2.public_key.as_str());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
