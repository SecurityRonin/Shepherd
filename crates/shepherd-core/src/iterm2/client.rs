use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_tungstenite::tungstenite::Message;

// Include prost-generated types. Named after the proto package ("iterm2").
#[allow(clippy::all)]
pub mod iterm2 {
    include!(concat!(env!("OUT_DIR"), "/iterm2.rs"));
}

static MSG_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> i64 {
    MSG_ID.fetch_add(1, Ordering::Relaxed) as i64
}

/// Abstraction over the WebSocket connection to iTerm2.
/// Implemented by WsClient (production) and MockTransport (tests in other modules).
#[async_trait::async_trait]
pub trait Iterm2Transport: Send {
    async fn send_recv(
        &mut self,
        msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<iterm2::ServerOriginatedMessage>;

    /// Send a message without waiting for a response (for subscriptions and
    /// fire-and-forget requests like SendTextRequest).
    async fn send_only(
        &mut self,
        msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<()>;

    /// Receive the next unsolicited server message (notifications).
    async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage>;
}

/// Production WebSocket client connecting over a Unix domain socket.
pub struct WsClient {
    ws: tokio_tungstenite::WebSocketStream<tokio::net::UnixStream>,
}

// tarpaulin-start-ignore
impl WsClient {
    /// Connect to iTerm2 over the Unix domain socket.
    pub async fn connect(
        socket_path: &std::path::Path,
        auth: &crate::iterm2::auth::Iterm2Auth,
    ) -> anyhow::Result<Self> {
        let stream = tokio::net::UnixStream::connect(socket_path)
            .await
            .with_context(|| format!("connecting to iTerm2 socket {}", socket_path.display()))?;

        let req = http::Request::builder()
            .uri("ws://localhost/")
            .header("x-iterm2-cookie", &auth.cookie)
            .header("x-iterm2-key", &auth.key)
            .body(())
            .context("building WebSocket handshake request")?;

        let (ws, _) = tokio_tungstenite::client_async_with_config(req, stream, None)
            .await
            .context("WebSocket handshake with iTerm2")?;

        Ok(Self { ws })
    }
}

#[async_trait::async_trait]
impl Iterm2Transport for WsClient {
    async fn send_recv(
        &mut self,
        mut msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
        msg.id = Some(next_id());
        let bytes = msg.encode_to_vec();
        self.ws
            .send(Message::Binary(bytes))
            .await
            .context("sending iTerm2 request")?;

        loop {
            let frame = self
                .ws
                .next()
                .await
                .context("iTerm2 connection closed")?
                .context("iTerm2 WebSocket error")?;
            if let Message::Binary(payload) = frame {
                let resp: iterm2::ServerOriginatedMessage =
                    ProstMessage::decode(payload.as_slice())
                        .context("decoding iTerm2 ServerOriginatedMessage")?;
                return Ok(resp);
            }
        }
    }

    async fn send_only(
        &mut self,
        mut msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<()> {
        msg.id = Some(next_id());
        let bytes = msg.encode_to_vec();
        self.ws
            .send(Message::Binary(bytes))
            .await
            .context("sending iTerm2 fire-and-forget message")?;
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
        loop {
            let frame = self
                .ws
                .next()
                .await
                .context("iTerm2 connection closed")?
                .context("iTerm2 WebSocket error")?;
            if let Message::Binary(payload) = frame {
                return ProstMessage::decode(payload.as_slice())
                    .context("decoding iTerm2 notification");
            }
        }
    }
}
// tarpaulin-stop-ignore

/// Discover the iTerm2 Unix socket path by globbing.
pub fn find_socket() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let pattern = format!(
        "{}/Library/Application Support/iTerm2/iterm2-daemon-*.socket",
        home
    );
    let mut matches: Vec<_> = glob::glob(&pattern)
        .context("globbing iTerm2 socket")?
        .filter_map(Result::ok)
        .collect();
    matches.sort();
    matches
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("iTerm2 socket not found — is iTerm2 running?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_socket_returns_result() {
        // In CI / test environments there's no iTerm2 socket — verify it
        // returns an error gracefully rather than panicking.
        let result = find_socket();
        // Either Ok (if tester has iTerm2 running) or Err — both are valid.
        // The important thing is no panic.
        let _ = result;
    }

    #[test]
    fn test_find_socket_glob_pattern_is_correct() {
        // Smoke-test that the glob pattern is parseable (no glob syntax errors).
        let home = std::env::var("HOME").unwrap_or_default();
        let pattern = format!(
            "{}/Library/Application Support/iTerm2/iterm2-daemon-*.socket",
            home
        );
        let iter = glob::glob(&pattern);
        assert!(iter.is_ok(), "glob pattern must be valid");
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let req = iterm2::ClientOriginatedMessage {
            id: Some(42),
            submessage: Some(
                iterm2::client_originated_message::Submessage::ListSessionsRequest(
                    iterm2::ListSessionsRequest {},
                ),
            ),
        };
        let encoded = ProstMessage::encode_to_vec(&req);
        let decoded: iterm2::ClientOriginatedMessage =
            ProstMessage::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded.id, Some(42));
        assert!(matches!(
            decoded.submessage,
            Some(iterm2::client_originated_message::Submessage::ListSessionsRequest(_))
        ));
    }
}
