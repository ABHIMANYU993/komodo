use anyhow::Context;
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Debug, Clone)]
pub struct ConnectionIdentifiers {
    pub host: String,
    pub query: String,
    pub accept: String,
}

impl ConnectionIdentifiers {
    pub fn new(host: String, query: String, accept: String) -> Self {
        Self { host, query, accept }
    }

    /// Extract host from a given WebSocket URL (including port if non-standard).
    pub fn from_url(raw_url: &str, query_param: &str, accept_header: &str) -> anyhow::Result<Self> {
        let parsed = Url::parse(raw_url).context("Failed to parse core_url")?;
        let host = parsed.host_str().context("URL has no host")?.to_string();
        let port_suffix = match parsed.port() {
            Some(p) => format!(":{p}"),
            None => String::new(),
        };
        let full_host = format!("{host}{port_suffix}");
        let query = format!("server={}", urlencoding::encode(query_param));
        Ok(Self {
            host: full_host,
            query,
            accept: accept_header.to_string(),
        })
    }

    /// Computes the exact SHA-256 prologue hash expected by Komodo Core:
    /// SHA-256("noise-wss-v1|" + host + "|" + query + "|" + accept + "|" + nonce)
    pub fn hash(&self, nonce: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"noise-wss-v1|");
        hasher.update(self.host.as_bytes());
        hasher.update(b"|");
        hasher.update(self.query.as_bytes());
        hasher.update(b"|");
        hasher.update(self.accept.as_bytes());
        hasher.update(b"|");
        hasher.update(nonce);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_trust_prologue_construction() {
        let id = ConnectionIdentifiers {
            host: "192.168.1.100:8120".to_string(),
            query: "server=Android_Node".to_string(),
            accept: "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".to_string(),
        };

        let nonce = [1u8; 32];
        let hash = id.hash(&nonce);
        assert_ne!(hash, [0u8; 32]);

        // Same input must produce exact same hash
        let hash2 = id.hash(&nonce);
        assert_eq!(hash, hash2);

        // Different nonce produces different hash
        let diff_nonce = [2u8; 32];
        assert_ne!(hash, id.hash(&diff_nonce));
    }
}
