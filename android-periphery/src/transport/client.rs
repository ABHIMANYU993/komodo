use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, anyhow};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::auth::keys::IdentityKeys;
use crate::auth::noise::{PeripheryNoiseClient, WebSocketTransportChannel};
use crate::capabilities::detector::DeviceCapabilities;
use crate::collectors::TelemetryEngine;
use crate::config::AgentConfig;
use crate::protocol::envelope::{RawTransportMessage, ResponseStatus};
use crate::protocol::login::LoginMessage;
use crate::protocol::request::PeripheryRequest;
use crate::protocol::response::PeripheryResponse;
use crate::protocol::types::{
    GetHealthResponse, GetVersionResponse, PeripheryInformation, PollStatusResponse,
};
use crate::terminal::TerminalManager;
use crate::transport::zero_trust::ConnectionIdentifiers;

pub struct CoreConnectionLoop {
    config: AgentConfig,
    keys: IdentityKeys,
    capabilities: DeviceCapabilities,
    telemetry: TelemetryEngine,
    terminal_mgr: Arc<TerminalManager>,
}

impl CoreConnectionLoop {
    pub fn new(
        config: AgentConfig,
        keys: IdentityKeys,
        capabilities: DeviceCapabilities,
        telemetry: TelemetryEngine,
    ) -> Self {
        Self {
            config,
            keys,
            capabilities,
            telemetry,
            terminal_mgr: Arc::new(TerminalManager::new()),
        }
    }

    /// Start the daemon connection supervisor loop. Never terminates; reconnects forever.
    pub async fn run(&self) {
        let mut already_logged_connection_error = false;

        loop {
            match self.connect_and_serve().await {
                Ok(()) => {
                    info!("Core connection terminated cleanly. Reconnecting in {}s...", self.config.reconnect_seconds);
                    already_logged_connection_error = false;
                }
                Err(e) => {
                    if !already_logged_connection_error {
                        warn!("Core connection failed: {e:#}. Retrying in {}s...", self.config.reconnect_seconds);
                        already_logged_connection_error = true;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(self.config.reconnect_seconds)).await;
        }
    }

    async fn connect_and_serve(&self) -> anyhow::Result<()> {
        let query = format!("server={}", urlencoding::encode(&self.config.connect_as));
        let trimmed_url = self.config.core_url.trim_end_matches('/');
        let endpoint = format!("{trimmed_url}/ws/periphery?{query}");

        debug!("Connecting outbound WebSocket to {endpoint}...");

        let request = endpoint.into_client_request()?;

        // Connect WebSocket
        let (ws_stream, response) = tokio_tungstenite::connect_async(request)
            .await
            .context("WebSocket TCP/TLS connection failed")?;

        let accept_header = response
            .headers()
            .get("sec-websocket-accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default()
            .to_string();

        let identifiers = ConnectionIdentifiers::from_url(
            &self.config.core_url,
            &self.config.connect_as,
            &accept_header,
        )?;

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Wrap stream into channel adapter for Noise handshake
        let mut auth_adapter = WsAuthAdapter {
            write: &mut ws_write,
            read: &mut ws_read,
        };

        // 1. Receive LoginMessage::OnboardingFlow(bool)
        let onboarding_flow = match auth_adapter.recv_msg().await? {
            LoginMessage::OnboardingFlow(flag) => flag,
            other => return Err(anyhow!("Expected LoginMessage::OnboardingFlow, got {other:?}")),
        };

        if onboarding_flow {
            info!("Core requested Onboarding Flow for '{}'", self.config.connect_as);

            let Some(ref onboarding_token) = self.config.onboarding_key else {
                return Err(anyhow!(
                    "Server '{}' is not registered on Core, and no 'onboarding_key' was configured",
                    self.config.connect_as
                ));
            };

            // Execute Noise handshake using onboarding token
            PeripheryNoiseClient::execute_handshake(
                &mut auth_adapter,
                onboarding_token,
                &identifiers,
                None,
            )
            .await
            .context("Onboarding Noise handshake failed")?;

            // Post-Noise Onboarding: Send persistent SPKI public key
            auth_adapter
                .send_msg(LoginMessage::PublicKey(self.keys.public_key.as_str().to_string()))
                .await
                .context("Failed to send public key during onboarding")?;

            // Receive final Success from Core
            match auth_adapter.recv_msg().await? {
                LoginMessage::Success => {
                    info!("Onboarding for '{}' completed successfully! Core will close socket to trigger standard login.", self.config.connect_as);
                    return Ok(()); // Returning Ok causes the loop to reconnect immediately for standard login
                }
                other => return Err(anyhow!("Expected Success after onboarding, got {other:?}")),
            }
        }

        // Standard Login Flow
        info!("Initiating standard Noise XX mutual authentication for '{}'...", self.config.connect_as);

        let allowed_keys: Option<&[String]> = if self.config.core_public_keys.is_empty() {
            None
        } else {
            Some(&self.config.core_public_keys)
        };

        PeripheryNoiseClient::execute_handshake(
            &mut auth_adapter,
            &self.keys.private_key,
            &identifiers,
            allowed_keys,
        )
        .await
        .context("Standard login Noise authentication failed")?;

        info!("Authenticated successfully as '{}' ✅ Entering message loop", self.config.connect_as);

        // Transition to normal post-authentication message loop
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(128);

        // Task A: Outbound writer & 5s Ping loop
        let writer_task = tokio::spawn(async move {
            loop {
                let msg_res = tokio::time::timeout(Duration::from_secs(5), outbound_rx.recv()).await;
                match msg_res {
                    Ok(Some(bytes)) => {
                        if let Err(e) = ws_write.send(Message::Binary(bytes.into())).await {
                            warn!("Failed to send binary frame: {e}");
                            break;
                        }
                    }
                    Ok(None) => break, // Channel closed
                    Err(_) => {
                        // 5 seconds idle: send standard WebSocket Ping frame
                        if let Err(e) = ws_write.send(Message::Ping(Vec::new().into())).await {
                            warn!("Failed to send ping frame: {e}");
                            break;
                        }
                    }
                }
            }
        });

        // Task B: Inbound reader & request dispatcher
        let telemetry_snapshot = self.telemetry.snapshot();
        let capabilities = self.capabilities.clone();
        let terminal_mgr = self.terminal_mgr.clone();
        let public_key_str = self.keys.public_key.as_str().to_string();

        while let Some(msg_result) = ws_read.next().await {
            let msg = msg_result.context("WebSocket read error")?;
            match msg {
                Message::Binary(data) => {
                    let raw_bytes = data.to_vec();
                    let Ok(parsed) = RawTransportMessage::decode(raw_bytes) else {
                        warn!("Received invalid wire envelope, ignoring");
                        continue;
                    };

                    match parsed {
                        RawTransportMessage::Request { channel, payload } => {
                            let tx = outbound_tx.clone();
                            let tele = telemetry_snapshot.clone();
                            let caps = capabilities.clone();
                            let term = terminal_mgr.clone();
                            let pubkey = public_key_str.clone();

                            tokio::spawn(async move {
                                Self::handle_core_request(channel, payload, tx, tele, caps, term, pubkey).await;
                            });
                        }
                        RawTransportMessage::Terminal { channel, status: _, payload } => {
                            terminal_mgr.forward_stdin(channel, payload).await;
                        }
                        _ => {}
                    }
                }
                Message::Ping(_) => {
                    // Handled automatically by tungstenite
                }
                Message::Close(_) => {
                    info!("Core closed WebSocket connection");
                    break;
                }
                _ => {}
            }
        }

        let _ = writer_task.await;
        Ok(())
    }

    async fn handle_core_request(
        channel: Uuid,
        payload: Vec<u8>,
        tx: mpsc::Sender<Vec<u8>>,
        telemetry: Arc<tokio::sync::RwLock<crate::collectors::TelemetrySnapshot>>,
        capabilities: DeviceCapabilities,
        terminal_mgr: Arc<TerminalManager>,
        public_key_str: String,
    ) {
        let req = match PeripheryRequest::parse(&payload) {
            Ok(r) => r,
            Err(e) => {
                let err_frame = PeripheryResponse::err(channel, &format!("Invalid request JSON: {e}"));
                let _ = tx.send(err_frame).await;
                return;
            }
        };

        // Long-running operations must send Pending keepalive every 4 seconds
        let tx_pending = tx.clone();
        let keepalive_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(4)).await;
                let pending_frame = PeripheryResponse::pending(channel);
                if tx_pending.send(pending_frame).await.is_err() {
                    break;
                }
            }
        });

        match req {
            PeripheryRequest::PollStatus(poll_params) => {
                let snapshot = telemetry.read().await;

                let periphery_info = PeripheryInformation {
                    version: capabilities.agent_version.clone(),
                    public_key: public_key_str,
                    terminals_disabled: false,
                    container_terminals_disabled: true,
                    stats_polling_rate: "1s".to_string(),
                    docker_connected: false,
                    public_ip: None,
                };

                let response = PollStatusResponse {
                    periphery_info,
                    system_info: capabilities.to_system_info(),
                    system_stats: if poll_params.include_stats {
                        Some(snapshot.stats.clone())
                    } else {
                        None
                    },
                    docker: None, // No docker on Android -> returns clean empty lists
                };

                if let Ok(frame) = PeripheryResponse::ok(channel, &response) {
                    let _ = tx.send(frame).await;
                }
            }
            PeripheryRequest::GetHealth => {
                let response = GetHealthResponse {};
                if let Ok(frame) = PeripheryResponse::ok(channel, &response) {
                    let _ = tx.send(frame).await;
                }
            }
            PeripheryRequest::GetVersion => {
                let response = GetVersionResponse {
                    version: capabilities.agent_version.clone(),
                };
                if let Ok(frame) = PeripheryResponse::ok(channel, &response) {
                    let _ = tx.send(frame).await;
                }
            }
            PeripheryRequest::GetSystemProcesses => {
                let snapshot = telemetry.read().await;
                if let Ok(frame) = PeripheryResponse::ok(channel, &snapshot.processes) {
                    let _ = tx.send(frame).await;
                }
            }
            PeripheryRequest::CreateServerTerminal { name, command, recreate: _ } => {
                match terminal_mgr.create_terminal(name, command).await {
                    Ok((session_id, mut stdout_rx)) => {
                        let now_epoch = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        let term_entity = serde_json::json!({
                            "id": session_id,
                            "name": format!("term-{}", &session_id.to_string()[..8]),
                            "command": "/system/bin/sh",
                            "created_at": now_epoch.to_string(),
                        });

                        if let Ok(frame) = PeripheryResponse::ok(channel, &term_entity) {
                            let _ = tx.send(frame).await;
                        }

                        // Stream stdout back to Core as Terminal frames (0x03)
                        let tx_stream = tx.clone();
                        tokio::spawn(async move {
                            while let Some(bytes) = stdout_rx.recv().await {
                                let terminal_frame = RawTransportMessage::Terminal {
                                    channel: session_id,
                                    status: ResponseStatus::Ok,
                                    payload: bytes,
                                };
                                if tx_stream.send(terminal_frame.encode()).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        let frame = PeripheryResponse::err(channel, &format!("Failed to spawn root PTY: {e}"));
                        let _ = tx.send(frame).await;
                    }
                }
            }
            PeripheryRequest::ConnectTerminal { terminal, target: _ } => {
                if let Ok(session_id) = Uuid::parse_str(&terminal) {
                    terminal_mgr.connect_channel(channel, session_id).await;
                    if let Ok(frame) = PeripheryResponse::ok(channel, &session_id) {
                        let _ = tx.send(frame).await;
                    }
                } else {
                    let frame = PeripheryResponse::err(channel, "Invalid terminal UUID");
                    let _ = tx.send(frame).await;
                }
            }
            PeripheryRequest::DisconnectTerminal { channel: term_ch } => {
                terminal_mgr.close_session(term_ch).await;
                let frame = PeripheryResponse::ok(channel, &serde_json::json!({})).unwrap();
                let _ = tx.send(frame).await;
            }
            PeripheryRequest::DeleteTerminal { terminal, target: _ } => {
                if let Ok(session_id) = Uuid::parse_str(&terminal) {
                    terminal_mgr.close_session(session_id).await;
                }
                let frame = PeripheryResponse::ok(channel, &serde_json::json!({})).unwrap();
                let _ = tx.send(frame).await;
            }
            PeripheryRequest::ListTerminals { target: _ } => {
                let empty: Vec<serde_json::Value> = Vec::new();
                let frame = PeripheryResponse::ok(channel, &empty).unwrap();
                let _ = tx.send(frame).await;
            }
            PeripheryRequest::Unsupported(op) => {
                let frame = PeripheryResponse::err(channel, &format!("Operation '{op}' is not supported on Android Periphery"));
                let _ = tx.send(frame).await;
            }
            _ => {
                let frame = PeripheryResponse::err(channel, "Unsupported operation");
                let _ = tx.send(frame).await;
            }
        }

        keepalive_task.abort();
    }
}

/// Adapter implementing `WebSocketTransportChannel` over split Tungstenite read/write streams.
struct WsAuthAdapter<'a, W, R> {
    write: &'a mut W,
    read: &'a mut R,
}

impl<W, R> WebSocketTransportChannel for WsAuthAdapter<'_, W, R>
where
    W: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    async fn send_msg(&mut self, msg: LoginMessage) -> anyhow::Result<()> {
        let wire = msg.encode();
        self.write
            .send(Message::Binary(wire.into()))
            .await
            .context("Failed to send WebSocket login message")?;
        Ok(())
    }

    async fn recv_msg(&mut self) -> anyhow::Result<LoginMessage> {
        while let Some(res) = self.read.next().await {
            let msg = res.context("Failed to read WebSocket login message")?;
            if let Message::Binary(data) = msg {
                let raw = RawTransportMessage::decode(data.to_vec())?;
                if let RawTransportMessage::Login(payload) = raw {
                    return LoginMessage::decode_payload(payload);
                }
            }
        }
        Err(anyhow!("WebSocket closed during login flow"))
    }
}
