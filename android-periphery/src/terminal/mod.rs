pub mod pty;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};
use uuid::Uuid;

use pty::PtySession;

pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<Uuid, PtySession>>>,
    channel_mappings: Arc<RwLock<HashMap<Uuid, Uuid>>>, // channel_id -> session_id
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            channel_mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create and start a new server terminal. Returns session UUID and stdout receiver.
    pub async fn create_terminal(
        &self,
        name: Option<String>,
        command: Option<String>,
    ) -> anyhow::Result<(Uuid, mpsc::Receiver<Vec<u8>>)> {
        let session_id = Uuid::new_v4();
        let term_name = name.unwrap_or_else(|| format!("term-{}", &session_id.to_string()[..8]));

        let (session, stdout_rx) = PtySession::spawn(session_id, term_name, command.as_deref())?;

        let mut lock = self.sessions.write().await;
        lock.insert(session_id, session);

        info!("Created new root PTY terminal session {}", session_id);
        Ok((session_id, stdout_rx))
    }

    /// Map a Core communication channel UUID to a terminal session UUID.
    pub async fn connect_channel(&self, channel: Uuid, session_id: Uuid) {
        let mut map = self.channel_mappings.write().await;
        map.insert(channel, session_id);
        debug!("Mapped channel {} to terminal session {}", channel, session_id);
    }

    /// Forward incoming stdin binary data from Core to the associated PTY session.
    pub async fn forward_stdin(&self, channel: Uuid, data: Vec<u8>) {
        let session_id = {
            let map = self.channel_mappings.read().await;
            map.get(&channel).copied()
        };

        if let Some(sid) = session_id {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(&sid) {
                let _ = session.stdin_tx.send(data).await;
            }
        } else {
            // Direct write if channel is session_id
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(&channel) {
                let _ = session.stdin_tx.send(data).await;
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
    }
}
