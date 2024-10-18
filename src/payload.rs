use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum Payload {
    Raw(Vec<u8>),
    Json(serde_json::Value),
    Error(String),
}

impl Payload {
    pub fn content_type(&self) -> Option<&str> {
        match &self {
            Payload::Json(_) => Some("application/json"),
            _ => None,
        }
    }

    pub fn from_bytes(bytes: Vec<u8>, content_type: Option<&str>) -> Option<Payload> {
        match content_type {
            Some(ct) => {
                if ct.contains("application/json") {
                    match serde_json::from_slice(&bytes) {
                        Ok(v) => Some(Payload::Json(v)),
                        Err(e) => Some(Payload::Error(e.to_string())),
                    }
                } else {
                    Some(Payload::Raw(bytes))
                }
            }
            _ => None,
        }
    }

    pub fn to_json(&self) -> Result<serde_json::Value, String> {
        match &self {
            Payload::Json(value) => Ok(value.clone()),
            Payload::Raw(vec) => match std::str::from_utf8(vec) {
                Ok(s) => serde_json::to_value(s).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            },
            Payload::Error(e) => Err(e.clone()),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        match &self {
            Payload::Json(value) => match serde_json::to_string(value) {
                Ok(s) => Ok(s.into_bytes()),
                Err(e) => Err(e.to_string()),
            },
            Payload::Raw(s) => Ok(s.clone()), // it would be nice to be able to avoid this copy
            Payload::Error(e) => Err(e.clone()),
        }
    }

    pub fn len(&self) -> Option<usize> {
        match &self {
            Payload::Json(_) => None,
            Payload::Raw(s) => Some(s.len()),
            Payload::Error(e) => Some(e.len()),
        }
    }

    pub fn to_pwm_headers(&self) -> Vec<(&str, &str)> {
        match &self {
            Payload::Json(value) => {
                let mut vec: Vec<(&str, &str)> = vec![];
                if let serde_json::Value::Object(map) = value {
                    for (k, entry) in map {
                        match entry {
                            serde_json::Value::Array(vs) => {
                                for v in vs {
                                    if let serde_json::Value::String(s) = v {
                                        vec.push((k, s));
                                    }
                                }
                            }

                            // accept string values as well
                            serde_json::Value::String(s) => {
                                vec.push((k, s));
                            }

                            _ => {}
                        }
                    }
                }

                vec
            }
            _ => {
                // TODO
                log::debug!("NYI: converting payload into headers vector");
                vec![]
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum StringOrVec {
    String(String),
    Vec(Vec<String>),
}

pub fn from_pwm_headers(vec: Vec<(String, String)>) -> Payload {
    let mut map = BTreeMap::new();
    for (k, v) in vec {
        let lk = k.to_lowercase();
        if let Some(vs) = map.get_mut(&lk) {
            match vs {
                StringOrVec::String(s) => {
                    let ss = s.to_string();
                    map.insert(lk, StringOrVec::Vec(vec![ss, v]));
                }
                StringOrVec::Vec(vs) => {
                    vs.push(v);
                }
            };
        } else {
            map.insert(lk, StringOrVec::String(v));
        }
    }

    let value = serde_json::to_value(map).expect("serializable map");
    Payload::Json(value)
}

pub fn to_pwm_headers(payload: Option<&Payload>) -> Vec<(&str, &str)> {
    payload.map_or_else(Vec::new, |p| p.to_pwm_headers())
}

/// To use this result in proxy-wasm calls as an Option<&[u8]>, use:
/// `data::to_pwm_body(p).as_deref()`.
pub fn to_pwm_body(payload: Option<&Payload>) -> Result<Option<Box<[u8]>>, String> {
    match payload {
        Some(p) => match p.to_bytes() {
            Ok(b) => Ok(Some(Vec::into_boxed_slice(b))),
            Err(e) => Err(e),
        },
        None => Ok(None),
    }
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

pub fn to_json_error_body(message: &str, request_id: Option<Vec<u8>>) -> String {
    serde_json::to_value(ErrorMessage {
        message,
        request_id: match request_id {
            Some(vec) => std::str::from_utf8(&vec).map(|v| v.to_string()).ok(),
            None => None,
        },
    })
    .ok()
    .map(|v| v.to_string())
    .expect("JSON error object")
}
