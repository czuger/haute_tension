//! The session cookie: a signed play-through id, and nothing else.
//!
//! The cookie carries the id of the game the reader is on and whatever is
//! waiting to be said to him on the next page. Everything else is loaded from
//! the database on each request, so two tabs of the same browser are the same
//! hero and nothing about him lives in the cookie.
//!
//! It is signed, not encrypted: its contents are readable, and the signature is
//! what keeps a reader from writing another hero's number into it.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;

use crate::application::templates::Flash;

/// The cookie the session travels in.
pub const COOKIE_NAME: &str = "session";

/// Where the messages waiting for the reader are kept.
const FLASHES_KEY: &str = "_flashes";

/// What the reader's browser carries between two requests.
#[derive(Clone, Debug, Default)]
pub struct Session {
    values: Map<String, Value>,
    changed: bool,
}

impl Session {
    /// Read the session out of a `Cookie` header, an unsigned one counting as none.
    ///
    /// A cookie whose signature does not check out is dropped whole rather than
    /// half-read: it was not written here.
    pub fn decode(cookie_header: Option<&str>, secret: &[u8]) -> Self {
        let Some(raw) = cookie_header.and_then(|header| named_cookie(header, COOKIE_NAME)) else {
            return Self::default();
        };
        let Some((payload, signature)) = raw.rsplit_once('.') else {
            return Self::default();
        };
        if !signed_with(payload, signature, secret) {
            return Self::default();
        }
        let Ok(decoded) = URL_SAFE_NO_PAD.decode(payload) else {
            return Self::default();
        };
        match serde_json::from_slice::<Value>(&decoded) {
            Ok(Value::Object(values)) => Self {
                values,
                changed: false,
            },
            _ => Self::default(),
        }
    }

    /// The session as a `Set-Cookie` value, signed.
    pub fn encode(&self, secret: &[u8]) -> String {
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&Value::Object(self.values.clone())).unwrap_or_default(),
        );
        let signature = sign(&payload, secret);
        format!(
            "{COOKIE_NAME}={payload}.{signature}; Path=/; HttpOnly; SameSite=Lax"
        )
    }

    /// Whether anything was written into the session this request.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// One value of the session, whatever shape it was written in.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key).filter(|value| !value.is_null())
    }

    /// One value of the session read as a play-through id.
    ///
    /// A cookie written before migration 001 carries a uuid, and a forged one
    /// carries anything at all: neither is a number, and neither names a game.
    pub fn get_id(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.parse().ok(),
            _ => None,
        }
    }

    /// Write one value into the session.
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
        self.changed = true;
    }

    /// Take one value out of the session.
    pub fn remove(&mut self, key: &str) {
        if self.values.remove(key).is_some() {
            self.changed = true;
        }
    }

    /// Leave something to be said to the reader on the next page he is shown.
    pub fn flash(&mut self, message: &str, category: &str) {
        let mut waiting = match self.values.get(FLASHES_KEY) {
            Some(Value::Array(waiting)) => waiting.clone(),
            _ => Vec::new(),
        };
        waiting.push(Value::Array(vec![
            Value::String(category.to_string()),
            Value::String(message.to_string()),
        ]));
        self.values
            .insert(FLASHES_KEY.to_string(), Value::Array(waiting));
        self.changed = true;
    }

    /// Everything waiting to be said, taken out of the session as it is read.
    ///
    /// Said once: a message that stayed would be shown on every later page.
    pub fn take_flashes(&mut self) -> Vec<Flash> {
        let Some(Value::Array(waiting)) = self.values.remove(FLASHES_KEY) else {
            return Vec::new();
        };
        self.changed = true;
        waiting
            .iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                Some(Flash {
                    category: pair.first()?.as_str()?.to_string(),
                    message: pair.get(1)?.as_str()?.to_string(),
                })
            })
            .collect()
    }
}

/// One named cookie out of a `Cookie` header.
fn named_cookie(header: &str, name: &str) -> Option<String> {
    header.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key.trim() == name).then(|| value.trim().to_string())
    })
}

/// The signature of a payload.
fn sign(payload: &str, secret: &[u8]) -> String {
    let mut signer =
        Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes a key of any length");
    signer.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(signer.finalize().into_bytes())
}

/// Whether a payload carries the signature this key would give it.
fn signed_with(payload: &str, signature: &str, secret: &[u8]) -> bool {
    let mut signer =
        Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes a key of any length");
    signer.update(payload.as_bytes());
    URL_SAFE_NO_PAD
        .decode(signature)
        .is_ok_and(|given| signer.verify_slice(&given).is_ok())
}
