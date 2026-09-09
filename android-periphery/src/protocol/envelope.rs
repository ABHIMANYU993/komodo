use anyhow::{Context, anyhow};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransportMessageVariant {
    Login = 0,
    Request = 1,
    Response = 2,
    Terminal = 3,
}

impl TransportMessageVariant {
    pub fn from_byte(byte: u8) -> anyhow::Result<Self> {
        match byte {
            0 => Ok(Self::Login),
            1 => Ok(Self::Request),
            2 => Ok(Self::Response),
            3 => Ok(Self::Terminal),
            other => Err(anyhow!("Unrecognized TransportMessageVariant byte: {other}")),
        }
    }

    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseStatus {
    Ok = 0,
    Err = 1,
    Pending = 2,
}

impl ResponseStatus {
    pub fn from_byte(byte: u8) -> anyhow::Result<Self> {
        match byte {
            0 => Ok(Self::Ok),
            1 => Ok(Self::Err),
            2 => Ok(Self::Pending),
            other => Err(anyhow!("Unrecognized ResponseStatus byte: {other}")),
        }
    }

    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

/// A parsed transport envelope from the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawTransportMessage {
    Login(Vec<u8>),
    Request {
        channel: Uuid,
        payload: Vec<u8>,
    },
    Response {
        channel: Uuid,
        status: ResponseStatus,
        payload: Vec<u8>,
    },
    Terminal {
        channel: Uuid,
        status: ResponseStatus,
        payload: Vec<u8>,
    },
}

impl RawTransportMessage {
    /// Decode a binary WebSocket frame into a RawTransportMessage.
    /// Format: [Payload] + [Variant (1B)]
    pub fn decode(mut bytes: Vec<u8>) -> anyhow::Result<Self> {
        let variant_byte = bytes.pop().context("Failed to decode message | bytes are empty")?;
        let variant = TransportMessageVariant::from_byte(variant_byte)?;

        match variant {
            TransportMessageVariant::Login => Ok(RawTransportMessage::Login(bytes)),
            TransportMessageVariant::Request => {
                if bytes.len() < 16 {
                    return Err(anyhow!(
                        "Request frame too short for Channel UUID: length {}",
                        bytes.len()
                    ));
                }
                let split_idx = bytes.len() - 16;
                let uuid_bytes = &bytes[split_idx..];
                let channel = Uuid::from_slice(uuid_bytes)
                    .context("Failed to parse Request Channel UUID")?;
                bytes.truncate(split_idx);
                Ok(RawTransportMessage::Request {
                    channel,
                    payload: bytes,
                })
            }
            TransportMessageVariant::Response => {
                if bytes.len() < 17 {
                    return Err(anyhow!(
                        "Response frame too short for Status and Channel UUID: length {}",
                        bytes.len()
                    ));
                }
                // Layout: [Data] + [Status (1B)] + [Channel UUID (16B)]
                let uuid_start = bytes.len() - 16;
                let channel = Uuid::from_slice(&bytes[uuid_start..])
                    .context("Failed to parse Response Channel UUID")?;
                let status_byte = bytes[uuid_start - 1];
                let status = ResponseStatus::from_byte(status_byte)?;
                bytes.truncate(uuid_start - 1);
                Ok(RawTransportMessage::Response {
                    channel,
                    status,
                    payload: bytes,
                })
            }
            TransportMessageVariant::Terminal => {
                if bytes.len() < 17 {
                    return Err(anyhow!(
                        "Terminal frame too short for Status and Channel UUID: length {}",
                        bytes.len()
                    ));
                }
                let uuid_start = bytes.len() - 16;
                let channel = Uuid::from_slice(&bytes[uuid_start..])
                    .context("Failed to parse Terminal Channel UUID")?;
                let status_byte = bytes[uuid_start - 1];
                let status = ResponseStatus::from_byte(status_byte)?;
                bytes.truncate(uuid_start - 1);
                Ok(RawTransportMessage::Terminal {
                    channel,
                    status,
                    payload: bytes,
                })
            }
        }
    }

    /// Encode a RawTransportMessage into wire bytes.
    pub fn encode(self) -> Vec<u8> {
        match self {
            RawTransportMessage::Login(mut payload) => {
                payload.push(TransportMessageVariant::Login.as_byte());
                payload
            }
            RawTransportMessage::Request { channel, mut payload } => {
                payload.extend_from_slice(channel.as_bytes());
                payload.push(TransportMessageVariant::Request.as_byte());
                payload
            }
            RawTransportMessage::Response {
                channel,
                status,
                mut payload,
            } => {
                payload.push(status.as_byte());
                payload.extend_from_slice(channel.as_bytes());
                payload.push(TransportMessageVariant::Response.as_byte());
                payload
            }
            RawTransportMessage::Terminal {
                channel,
                status,
                mut payload,
            } => {
                payload.push(status.as_byte());
                payload.extend_from_slice(channel.as_bytes());
                payload.push(TransportMessageVariant::Terminal.as_byte());
                payload
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_request_roundtrip() {
        let channel = Uuid::new_v4();
        let payload = b"{\"type\":\"GetHealth\",\"params\":{}}".to_vec();
        let original = RawTransportMessage::Request {
            channel,
            payload: payload.clone(),
        };

        let encoded = original.clone().encode();
        assert_eq!(encoded[encoded.len() - 1], 0x01); // Request variant
        assert_eq!(&encoded[encoded.len() - 17..encoded.len() - 1], channel.as_bytes());

        let decoded = RawTransportMessage::decode(encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_envelope_response_roundtrip() {
        let channel = Uuid::new_v4();
        let payload = b"{\"cpu\":12.5}".to_vec();
        let original = RawTransportMessage::Response {
            channel,
            status: ResponseStatus::Ok,
            payload: payload.clone(),
        };

        let encoded = original.clone().encode();
        assert_eq!(encoded[encoded.len() - 1], 0x02); // Response variant
        assert_eq!(&encoded[encoded.len() - 17..encoded.len() - 1], channel.as_bytes());
        assert_eq!(encoded[encoded.len() - 18], 0x00); // Status Ok

        let decoded = RawTransportMessage::decode(encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_envelope_pending_response() {
        let channel = Uuid::new_v4();
        let original = RawTransportMessage::Response {
            channel,
            status: ResponseStatus::Pending,
            payload: Vec::new(),
        };

        let encoded = original.clone().encode();
        assert_eq!(encoded.len(), 18); // 1B status + 16B UUID + 1B variant
        assert_eq!(encoded[17], 0x02); // Variant
        assert_eq!(encoded[0], 0x02);  // Status Pending

        let decoded = RawTransportMessage::decode(encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
