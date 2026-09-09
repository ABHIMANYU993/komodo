use std::time::Duration;
use tracing::debug;

pub struct IpResolver;

impl IpResolver {
    /// Attempts to query public IP via standard HTTP/1.0 with a tight 2-second timeout.
    /// If internet is unavailable or fails, falls back to local network IP.
    pub async fn resolve() -> Option<String> {
        // 1. Try public IP lookup (api.ipify.org port 80)
        if let Ok(ip) = Self::query_public_ip().await {
            let clean = ip.trim().to_string();
            if !clean.is_empty() && clean.contains('.') {
                debug!("Resolved public IP: {}", clean);
                return Some(clean);
            }
        }

        // 2. Fallback to local routable network IP (e.g. wlan0 192.168.31.x)
        if let Ok(local_ip) = Self::query_local_ip() {
            let clean = local_ip.trim().to_string();
            if !clean.is_empty() {
                debug!("Resolved local network IP fallback: {}", clean);
                return Some(clean);
            }
        }

        None
    }

    async fn query_public_ip() -> anyhow::Result<String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let res = tokio::time::timeout(Duration::from_secs(2), async {
            let mut stream = TcpStream::connect("api.ipify.org:80").await?;
            let req = "GET / HTTP/1.0\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n";
            stream.write_all(req.as_bytes()).await?;

            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await?;
            let resp = String::from_utf8_lossy(&buf);

            if let Some(body_start) = resp.find("\r\n\r\n") {
                let ip_str = &resp[body_start + 4..];
                Ok(ip_str.trim().to_string())
            } else {
                anyhow::bail!("Invalid HTTP response")
            }
        })
        .await??;

        Ok(res)
    }

    fn query_local_ip() -> anyhow::Result<String> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.connect("1.1.1.1:80")?;
        let addr = socket.local_addr()?;
        let ip = addr.ip().to_string();
        if ip != "0.0.0.0" && ip != "127.0.0.1" {
            Ok(ip)
        } else {
            anyhow::bail!("No valid routable interface")
        }
    }
}
