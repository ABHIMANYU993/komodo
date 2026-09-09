pub mod pty;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};
use uuid::Uuid;

use pty::PtySession;

#[derive(serde::Deserialize, Debug)]
struct TerminalResizeMessage {
    rows: u16,
    cols: u16,
}

pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<Uuid, PtySession>>>,
    name_mappings: Arc<RwLock<HashMap<String, Uuid>>>,
    channel_mappings: Arc<RwLock<HashMap<Uuid, Uuid>>>, // channel_id -> session_id
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            name_mappings: Arc::new(RwLock::new(HashMap::new())),
            channel_mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create and start a new server terminal. Returns (session UUID, name, stdout receiver).
    pub async fn create_terminal(
        &self,
        name: Option<String>,
        command: Option<String>,
    ) -> anyhow::Result<(Uuid, String, mpsc::Receiver<Vec<u8>>)> {
        let session_id = Uuid::new_v4();
        let term_name = name.unwrap_or_else(|| format!("term-{}", &session_id.to_string()[..8]));

        let (session, stdout_rx) = PtySession::spawn(session_id, term_name.clone(), command.as_deref())?;

        let mut lock = self.sessions.write().await;
        lock.insert(session_id, session);

        let mut names = self.name_mappings.write().await;
        names.insert(term_name.clone(), session_id);

        info!("Created new root PTY terminal session {} ({})", session_id, term_name);
        Ok((session_id, term_name, stdout_rx))
    }

    /// Retrieve existing session ID by name, or spawn a new one.
    pub async fn get_or_create(&self, name: &str, command: Option<String>) -> (Uuid, String) {
        {
            let names = self.name_mappings.read().await;
            if let Some(sid) = names.get(name) {
                return (*sid, name.to_string());
            }
        }

        if let Ok((sid, term_name, _)) = self.create_terminal(Some(name.to_string()), command).await {
            (sid, term_name)
        } else {
            (Uuid::new_v4(), name.to_string())
        }
    }

    /// Map a Core communication channel UUID to a terminal session UUID.
    pub async fn connect_channel(&self, channel: Uuid, session_id: Uuid) {
        let mut map = self.channel_mappings.write().await;
        map.insert(channel, session_id);
        debug!("Mapped channel {} to terminal session {}", channel, session_id);
    }

    /// Forward incoming stdin binary data from Core to the associated PTY session.
    pub async fn forward_stdin(&self, channel: Uuid, mut data: Vec<u8>) {
        let session_id = {
            let map = self.channel_mappings.read().await;
            map.get(&channel).copied().unwrap_or(channel)
        };

        if data.is_empty() {
            return;
        }

        // Komodo protocol: last byte is TerminalStdinMessageVariant:
        // 0x00 = Begin
        // 0x01 = Forward
        // 0x02 = Resize
        let variant_byte = data.pop().unwrap();
        match variant_byte {
            0x00 => {
                debug!("Received Terminal Begin trigger for channel {}", channel);
            }
            0x01 => {
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(&session_id) {
                    let _ = session.stdin_tx.send(data).await;
                }
            }
            0x02 => {
                if let Ok(resize) = serde_json::from_slice::<TerminalResizeMessage>(&data) {
                    let sessions = self.sessions.read().await;
                    if let Some(session) = sessions.get(&session_id) {
                        let _ = session.resize(resize.cols, resize.rows);
                        debug!("Resized terminal {} to {}x{}", session_id, resize.cols, resize.rows);
                    }
                }
            }
            _ => {
                // Fallback: raw data without variant byte
                data.push(variant_byte);
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(&session_id) {
                    let _ = session.stdin_tx.send(data).await;
                }
            }
        }
    }

    /// Disconnect and terminate a terminal session.
    pub async fn close_session(&self, session_id: Uuid) {
        let mut lock = self.sessions.write().await;
        if let Some(session) = lock.remove(&session_id) {
            session.close();
            info!("Closed PTY terminal session {}", session_id);
        }

        let mut map = self.channel_mappings.write().await;
        map.retain(|_, v| *v != session_id);

        let mut names = self.name_mappings.write().await;
        names.retain(|_, v| *v != session_id);
    }
}
