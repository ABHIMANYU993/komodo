use serde::Serialize;
use uuid::Uuid;
use super::envelope::{RawTransportMessage, ResponseStatus};

pub struct PeripheryResponse;

impl PeripheryResponse {
    /// Encode a successful response payload for a channel.
    pub fn ok<T: Serialize>(channel: Uuid, data: &T) -> anyhow::Result<Vec<u8>> {
        let payload = serde_json::to_vec(data)?;
        let raw = RawTransportMessage::Response {
            channel,
            status: ResponseStatus::Ok,
            payload,
        };
        Ok(raw.encode())
    }

    /// Encode an error response payload for a channel.
    pub fn err(channel: Uuid, error_message: &str) -> Vec<u8> {
        let err_json = serde_json::json!({
            "error": error_message,
        });
        let payload = serde_json::to_vec(&err_json).unwrap_or_else(|_| error_message.as_bytes().to_vec());
        let raw = RawTransportMessage::Response {
            channel,
            status: ResponseStatus::Err,
            payload,
        };
        raw.encode()
    }

    /// Encode a 4-second "Pending" in-progress keepalive frame for a channel.
    pub fn pending(channel: Uuid) -> Vec<u8> {
        let raw = RawTransportMessage::Response {
            channel,
            status: ResponseStatus::Pending,
            payload: Vec::new(),
        };
        raw.encode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_response_framing() {
        let channel = Uuid::new_v4();
        let wire = PeripheryResponse::ok(channel, &"healthy").unwrap();
        let decoded = RawTransportMessage::decode(wire).unwrap();
        match decoded {
            RawTransportMessage::Response {
                channel: ch,
                status,
                payload,
            } => {
                assert_eq!(ch, channel);
                assert_eq!(status, ResponseStatus::Ok);
                assert_eq!(String::from_utf8(payload).unwrap(), "\"healthy\"");
            }
            other => panic!("Expected Response, got {other:?}"),
        }
    }

    #[test]
    fn test_err_response_framing() {
        let channel = Uuid::new_v4();
        let wire = PeripheryResponse::err(channel, "Capability not supported on Android");
        let decoded = RawTransportMessage::decode(wire).unwrap();
        match decoded {
            RawTransportMessage::Response {
                channel: ch,
                status,
                payload,
            } => {
                assert_eq!(ch, channel);
                assert_eq!(status, ResponseStatus::Err);
                assert!(String::from_utf8(payload).unwrap().contains("Capability not supported"));
            }
            other => panic!("Expected Response, got {other:?}"),
        }
    }

    #[test]
    fn test_pending_response_framing() {
        let channel = Uuid::new_v4();
        let wire = PeripheryResponse::pending(channel);
        let decoded = RawTransportMessage::decode(wire).unwrap();
        match decoded {
            RawTransportMessage::Response {
                channel: ch,
                status,
                payload,
            } => {
                assert_eq!(ch, channel);
                assert_eq!(status, ResponseStatus::Pending);
                assert!(payload.is_empty());
            }
            other => panic!("Expected Response, got {other:?}"),
        }
    }
}
