use anyhow::{Context, anyhow};
use mogh_pki::{SpkiPublicKey, mutual::MutualNoiseHandshake};
use tracing::debug;
use crate::protocol::login::LoginMessage;
use crate::transport::zero_trust::ConnectionIdentifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseAuthSession {
    pub remote_public_key: String,
}

pub struct PeripheryNoiseClient;

impl PeripheryNoiseClient {
    /// Perform the complete Noise XX 3-way handshake as Initiator (Periphery)
    /// against an in-flight socket / channel communication loop.
    pub async fn execute_handshake<S>(
        socket: &mut S,
        private_key_pem: &str,
        identifiers: &ConnectionIdentifiers,
        expected_core_public_keys: Option<&[String]>,
    ) -> anyhow::Result<NoiseAuthSession>
    where
        S: WebSocketTransportChannel,
    {
        // 1. Receive connection Nonce from Core (Server)
        let nonce = match socket.recv_msg().await? {
            LoginMessage::Nonce(n) => n,
            LoginMessage::Handshake(_) => return Err(anyhow!("Received Handshake instead of Nonce")),
            other => return Err(anyhow!("Expected LoginMessage::Nonce, got: {other:?}")),
        };

        // 2. Initialize MutualNoiseHandshake as Initiator with zero-trust prologue
        let prologue = identifiers.hash(&nonce);
        let mut handshake = MutualNoiseHandshake::new_initiator(private_key_pem, &prologue)
            .context("Failed to initialize Noise XX initiator handshake")?;

        // 3. Generate handshake_m1 (-> e) and send to Core
        let m1 = handshake.next_message().context("Failed to generate Noise m1")?;
        socket.send_msg(LoginMessage::Handshake(m1)).await?;

        // 4. Receive handshake_m2 (<- e, ee, s, es) from Core
        let m2 = match socket.recv_msg().await? {
            LoginMessage::Handshake(bytes) => bytes,
            other => return Err(anyhow!("Expected LoginMessage::Handshake (m2), got: {other:?}")),
        };
        handshake.read_message(&m2).context("Failed to process Noise m2 from Core")?;

        // 5. Extract Core's public key and validate if expected_core_public_keys is configured
        let core_pubkey_raw = handshake.remote_public_key().context("Failed to get Core public key from Noise")?;
        let core_pubkey = SpkiPublicKey::from_raw_bytes(core_pubkey_raw)
            .context("Invalid Core public key format")?
            .into_inner();

        if let Some(allowed_keys) = expected_core_public_keys {
            if !allowed_keys.is_empty() && !allowed_keys.contains(&core_pubkey) {
                return Err(anyhow!(
                    "Untrusted Core public key: {core_pubkey}. Allowed keys: {allowed_keys:?}"
                ));
            }
        }

        // 6. Generate handshake_m3 (-> s, se) and send to Core
        let m3 = handshake.next_message().context("Failed to generate Noise m3")?;
        socket.send_msg(LoginMessage::Handshake(m3)).await?;

        // 7. Receive LoginMessage::Success from Core
        match socket.recv_msg().await? {
            LoginMessage::Success => {
                debug!("Noise XX handshake completed successfully with Core public key: {core_pubkey}");
                Ok(NoiseAuthSession {
                    remote_public_key: core_pubkey,
                })
            }
            other => Err(anyhow!("Expected LoginMessage::Success, got: {other:?}")),
        }
    }
}

/// Abstraction over sending and receiving LoginMessages for unit tests and WebSocket clients.
#[allow(async_fn_in_trait)]
pub trait WebSocketTransportChannel {
    async fn send_msg(&mut self, msg: LoginMessage) -> anyhow::Result<()>;
    async fn recv_msg(&mut self) -> anyhow::Result<LoginMessage>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::keys::IdentityKeys;

    struct InMemoryChannel {
        incoming: tokio::sync::mpsc::Receiver<LoginMessage>,
        outgoing: tokio::sync::mpsc::Sender<LoginMessage>,
    }

    impl WebSocketTransportChannel for InMemoryChannel {
        async fn send_msg(&mut self, msg: LoginMessage) -> anyhow::Result<()> {
            self.outgoing.send(msg).await.map_err(|e| anyhow!("Send error: {e}"))
        }

        async fn recv_msg(&mut self) -> anyhow::Result<LoginMessage> {
            self.incoming.recv().await.ok_or_else(|| anyhow!("Channel closed"))
        }
    }

    fn create_mock_pair() -> (InMemoryChannel, InMemoryChannel) {
        let (tx1, rx1) = tokio::sync::mpsc::channel(16);
        let (tx2, rx2) = tokio::sync::mpsc::channel(16);
        let client = InMemoryChannel { incoming: rx2, outgoing: tx1 };
        let server = InMemoryChannel { incoming: rx1, outgoing: tx2 };
        (client, server)
    }

    // Mock Core Responder task executing the exact ServerLoginFlow from upstream
    async fn mock_core_responder(
        mut server_chan: InMemoryChannel,
        core_private_key: String,
        identifiers: ConnectionIdentifiers,
        expected_client_pubkey: Option<String>,
    ) -> anyhow::Result<String> {
        let nonce = [77u8; 32];
        server_chan.send_msg(LoginMessage::Nonce(nonce)).await?;

        let prologue = identifiers.hash(&nonce);
        let mut handshake = MutualNoiseHandshake::new_responder(&core_private_key, &prologue)?;

        // Receive m1
        let m1 = match server_chan.recv_msg().await? {
            LoginMessage::Handshake(b) => b,
            other => return Err(anyhow!("Expected m1, got {other:?}")),
        };
        handshake.read_message(&m1)?;

        // Send m2
        let m2 = handshake.next_message()?;
        server_chan.send_msg(LoginMessage::Handshake(m2)).await?;

        // Receive m3
        let m3 = match server_chan.recv_msg().await? {
            LoginMessage::Handshake(b) => b,
            other => return Err(anyhow!("Expected m3, got {other:?}")),
        };
        handshake.read_message(&m3)?;

        let client_pubkey = SpkiPublicKey::from_raw_bytes(handshake.remote_public_key()?)?.into_inner();

        if let Some(expected) = expected_client_pubkey {
            if client_pubkey != expected {
                return Err(anyhow!("Unauthorized client public key: {client_pubkey}"));
            }
        }

        server_chan.send_msg(LoginMessage::Success).await?;
        Ok(client_pubkey)
    }

    #[tokio::test]
    async fn test_normal_registered_node_handshake() {
        let (mut client_chan, server_chan) = create_mock_pair();

        let client_keys = IdentityKeys::generate().unwrap();
        let client_priv = client_keys.private_key.clone();
        let client_pub = client_keys.public_key.clone().into_inner();

        let core_keys = IdentityKeys::generate().unwrap();
        let core_priv = core_keys.private_key;
        let core_pub = core_keys.public_key.into_inner();

        let identifiers = ConnectionIdentifiers {
            host: "localhost:8120".to_string(),
            query: "server=test_node".to_string(),
            accept: "test_accept_token".to_string(),
        };

        let ids_clone = identifiers.clone();
        let core_task = tokio::spawn(async move {
            mock_core_responder(server_chan, core_priv, ids_clone, Some(client_pub)).await
        });

        let session = PeripheryNoiseClient::execute_handshake(
            &mut client_chan,
            &client_priv,
            &identifiers,
            Some(&[core_pub.clone()]),
        )
        .await
        .unwrap();

        assert_eq!(session.remote_public_key, core_pub);
        let authenticated_client_key = core_task.await.unwrap().unwrap();
        assert_eq!(authenticated_client_key, client_keys.public_key.into_inner());
    }

    #[tokio::test]
    async fn test_onboarding_node_handshake() {
        let (mut client_chan, server_chan) = create_mock_pair();

        let onboarding_keys = IdentityKeys::generate().unwrap();
        let onboarding_priv = onboarding_keys.private_key;
        let onboarding_pub = onboarding_keys.public_key.into_inner();

        let core_keys = IdentityKeys::generate().unwrap();
        let core_priv = core_keys.private_key;
        let core_pub = core_keys.public_key.into_inner();

        let identifiers = ConnectionIdentifiers {
            host: "localhost:8120".to_string(),
            query: "server=onboarding_node".to_string(),
            accept: "test_accept_onboard".to_string(),
        };

        let ids_clone = identifiers.clone();
        let core_task = tokio::spawn(async move {
            mock_core_responder(server_chan, core_priv, ids_clone, Some(onboarding_pub)).await
        });

        let session = PeripheryNoiseClient::execute_handshake(
            &mut client_chan,
            &onboarding_priv,
            &identifiers,
            None,
        )
        .await
        .unwrap();

        assert_eq!(session.remote_public_key, core_pub);
        core_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_bad_credentials_rejected() {
        let (mut client_chan, server_chan) = create_mock_pair();

        let client_keys = IdentityKeys::generate().unwrap();
        let wrong_expected_pubkey = IdentityKeys::generate().unwrap().public_key.into_inner();

        let core_keys = IdentityKeys::generate().unwrap();
        let core_priv = core_keys.private_key;

        let identifiers = ConnectionIdentifiers {
            host: "localhost:8120".to_string(),
            query: "server=bad_cred_node".to_string(),
            accept: "test_accept".to_string(),
        };

        let ids_clone = identifiers.clone();
        let core_task = tokio::spawn(async move {
            mock_core_responder(server_chan, core_priv, ids_clone, Some(wrong_expected_pubkey)).await
        });

        let client_res = PeripheryNoiseClient::execute_handshake(
            &mut client_chan,
            &client_keys.private_key,
            &identifiers,
            None,
        )
        .await;

        let core_res = core_task.await.unwrap();
        assert!(core_res.is_err());
        assert!(core_res.unwrap_err().to_string().contains("Unauthorized client public key"));
        assert!(client_res.is_err());
    }

    #[tokio::test]
    async fn test_malformed_login_nonce() {
        let (mut client_chan, mut server_chan) = create_mock_pair();

        let client_keys = IdentityKeys::generate().unwrap();
        let identifiers = ConnectionIdentifiers {
            host: "localhost:8120".to_string(),
            query: "server=malformed".to_string(),
            accept: "test_accept".to_string(),
        };

        // Core sends invalid message variant instead of nonce
        tokio::spawn(async move {
            server_chan.send_msg(LoginMessage::Success).await.unwrap();
        });

        let res = PeripheryNoiseClient::execute_handshake(
            &mut client_chan,
            &client_keys.private_key,
            &identifiers,
            None,
        )
        .await;

        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Expected LoginMessage::Nonce"));
    }
}
