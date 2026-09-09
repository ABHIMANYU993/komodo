use anyhow::{Context, anyhow};
use super::envelope::{RawTransportMessage, ResponseStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginMessage {
    Success,
    Nonce([u8; 32]),
    Handshake(Vec<u8>),
    OnboardingFlow(bool),
    PublicKey(String),
    V1PasskeyFlow(bool),
    V1Passkey(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LoginMessageVariant {
    Success = 0,
    Nonce = 1,
    Handshake = 2,
    OnboardingFlow = 3,
    PublicKey = 4,
    V1PasskeyFlow = 5,
    V1Passkey = 6,
}

impl LoginMessageVariant {
    pub fn from_byte(byte: u8) -> anyhow::Result<Self> {
        match byte {
            0 => Ok(Self::Success),
            1 => Ok(Self::Nonce),
            2 => Ok(Self::Handshake),
            3 => Ok(Self::OnboardingFlow),
            4 => Ok(Self::PublicKey),
            5 => Ok(Self::V1PasskeyFlow),
            6 => Ok(Self::V1Passkey),
            other => Err(anyhow!("Unrecognized LoginMessageVariant byte: {other}")),
        }
    }

    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

impl LoginMessage {
    /// Encode a LoginMessage into full transport wire bytes (including login variant and transport variant).
    pub fn encode(self) -> Vec<u8> {
        let mut bytes = match &self {
            LoginMessage::Success => Vec::new(),
            LoginMessage::Nonce(nonce) => nonce.to_vec(),
            LoginMessage::Handshake(data) => data.clone(),
            LoginMessage::OnboardingFlow(flag) => vec![if *flag { 1 } else { 0 }],
            LoginMessage::PublicKey(key) => key.as_bytes().to_vec(),
            LoginMessage::V1PasskeyFlow(flag) => vec![if *flag { 1 } else { 0 }],
            LoginMessage::V1Passkey(data) => data.clone(),
        };

        let login_variant = match &self {
            LoginMessage::Success => LoginMessageVariant::Success,
            LoginMessage::Nonce(_) => LoginMessageVariant::Nonce,
            LoginMessage::Handshake(_) => LoginMessageVariant::Handshake,
            LoginMessage::OnboardingFlow(_) => LoginMessageVariant::OnboardingFlow,
            LoginMessage::PublicKey(_) => LoginMessageVariant::PublicKey,
            LoginMessage::V1PasskeyFlow(_) => LoginMessageVariant::V1PasskeyFlow,
            LoginMessage::V1Passkey(_) => LoginMessageVariant::V1Passkey,
        };

        bytes.push(login_variant.as_byte());
        // Response status Ok
        bytes.push(ResponseStatus::Ok.as_byte());
        RawTransportMessage::Login(bytes).encode()
    }

    /// Encode an error message into transport wire bytes.
    pub fn encode_error(err_json: &str) -> Vec<u8> {
        let mut bytes = err_json.as_bytes().to_vec();
        bytes.push(ResponseStatus::Err.as_byte());
        RawTransportMessage::Login(bytes).encode()
    }

    /// Decode raw payload bytes (from RawTransportMessage::Login) into a LoginMessage.
    pub fn decode_payload(mut bytes: Vec<u8>) -> anyhow::Result<Self> {
        let status_byte = bytes.pop().context("Failed to parse login message | empty bytes")?;
        let status = ResponseStatus::from_byte(status_byte)?;

        if status == ResponseStatus::Err {
            let err_str = String::from_utf8_lossy(&bytes);
            return Err(anyhow!("Received login error from remote: {err_str}"));
        }

        if status == ResponseStatus::Pending {
            return Err(anyhow!("Login message should not have Pending status"));
        }

        let variant_byte = bytes.pop().context("Failed to parse login message variant | bytes too short")?;
        let variant = LoginMessageVariant::from_byte(variant_byte)?;

        match variant {
            LoginMessageVariant::Success => Ok(LoginMessage::Success),
            LoginMessageVariant::Nonce => {
                if bytes.len() != 32 {
                    return Err(anyhow!("Invalid connection nonce length: {}", bytes.len()));
                }
                let mut nonce = [0u8; 32];
                nonce.copy_from_slice(&bytes);
                Ok(LoginMessage::Nonce(nonce))
            }
            LoginMessageVariant::Handshake => Ok(LoginMessage::Handshake(bytes)),
            LoginMessageVariant::OnboardingFlow => {
                match bytes.as_slice() {
                    [0] => Ok(LoginMessage::OnboardingFlow(false)),
                    [1] => Ok(LoginMessage::OnboardingFlow(true)),
                    other => Err(anyhow!("Unrecognized OnboardingFlow byte: {other:?}")),
                }
            }
            LoginMessageVariant::PublicKey => {
                let key = String::from_utf8(bytes).context("Public key is not valid UTF-8")?;
                Ok(LoginMessage::PublicKey(key))
            }
            LoginMessageVariant::V1PasskeyFlow => {
                match bytes.as_slice() {
                    [0] => Ok(LoginMessage::V1PasskeyFlow(false)),
                    [1] => Ok(LoginMessage::V1PasskeyFlow(true)),
                    other => Err(anyhow!("Unrecognized V1PasskeyFlow byte: {other:?}")),
                }
            }
            LoginMessageVariant::V1Passkey => Ok(LoginMessage::V1Passkey(bytes)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_nonce_roundtrip() {
        let nonce = [42u8; 32];
        let original = LoginMessage::Nonce(nonce);
        let wire = original.clone().encode();

        let raw = RawTransportMessage::decode(wire).unwrap();
        match raw {
            RawTransportMessage::Login(payload) => {
                let decoded = LoginMessage::decode_payload(payload).unwrap();
                assert_eq!(decoded, original);
            }
            other => panic!("Expected RawTransportMessage::Login, got {other:?}"),
        }
    }

    #[test]
    fn test_login_onboarding_flow_roundtrip() {
        for flag in [false, true] {
            let original = LoginMessage::OnboardingFlow(flag);
            let wire = original.clone().encode();
            let raw = RawTransportMessage::decode(wire).unwrap();
            if let RawTransportMessage::Login(payload) = raw {
                let decoded = LoginMessage::decode_payload(payload).unwrap();
                assert_eq!(decoded, original);
            }
        }
    }

    #[test]
    fn test_login_public_key_roundtrip() {
        let key = "MCowBQYDK2VuAyEA9+5e...".to_string();
        let original = LoginMessage::PublicKey(key);
        let wire = original.clone().encode();
        let raw = RawTransportMessage::decode(wire).unwrap();
        if let RawTransportMessage::Login(payload) = raw {
            let decoded = LoginMessage::decode_payload(payload).unwrap();
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn test_login_error_payload() {
        let wire = LoginMessage::encode_error("{\"error\":\"Invalid authentication key\"}");
        let raw = RawTransportMessage::decode(wire).unwrap();
        if let RawTransportMessage::Login(payload) = raw {
            let res = LoginMessage::decode_payload(payload);
            assert!(res.is_err());
            assert!(res.unwrap_err().to_string().contains("Invalid authentication key"));
        }
    }
}
