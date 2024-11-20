use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    Raw(Vec<u8>),
    Json(Json),
    Error(String),
}

pub const JSON_CONTENT_TYPE: &str = "application/json";

impl Payload {
    pub fn content_type(&self) -> Option<&str> {
        match &self {
            Payload::Json(_) => Some(JSON_CONTENT_TYPE),
            _ => None,
        }
    }

    pub fn from_bytes(bytes: Vec<u8>, content_type: Option<&str>) -> Option<Payload> {
        match content_type {
            Some(ct) => {
                if ct.contains(JSON_CONTENT_TYPE) {
                    match serde_json::from_slice(&bytes) {
                        Ok(v) => Some(Payload::Json(v)),
                        Err(e) => Some(Payload::Error(e.to_string())),
                    }
                } else if ct.contains("application/x-www-form-urlencoded") {
                    Some(Payload::Json(urlencoded_bytes_to_map(&bytes).into()))
                } else {
                    Some(Payload::Raw(bytes))
                }
            }
            _ => Some(Payload::Raw(bytes)),
        }
    }

    pub fn to_json(&self) -> Result<Json, String> {
        match &self {
            Payload::Json(value) => Ok(value.clone()),
            Payload::Raw(vec) => match std::str::from_utf8(vec) {
                Ok(s) => serde_json::to_value(s).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            },
            Payload::Error(e) => Err(e.clone()),
        }
    }

    pub fn to_bytes(&self, content_type: Option<&str>) -> Result<Vec<u8>, String> {
        let to_json = content_type.is_some_and(|ct| ct.contains(JSON_CONTENT_TYPE));

        match &self {
            Payload::Json(Json::String(string)) if !to_json => {
                // do not serialize a JSON string unless explicitly asked
                Ok(string.clone().into_bytes())
            }
            Payload::Json(value) => Ok(value.to_string().into_bytes()),
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
                if let Json::Object(map) = value {
                    for (k, entry) in map {
                        match entry {
                            Json::Array(vs) => {
                                for v in vs {
                                    if let Json::String(s) = v {
                                        vec.push((k, s));
                                    }
                                }
                            }

                            // accept string values as well
                            Json::String(s) => {
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

    pub fn to_pwm_query(&self) -> String {
        match &self {
            Payload::Json(value) => {
                let mut encoder = form_urlencoded::Serializer::new(String::new());
                match value {
                    serde_json::Value::Object(map) => {
                        for (k, entry) in map {
                            match entry {
                                serde_json::Value::Array(vs) => {
                                    for v in vs {
                                        if let serde_json::Value::String(s) = v {
                                            encoder.append_pair(k, s);
                                        } else {
                                            encoder.append_pair(k, &v.to_string());
                                        }
                                    }
                                }
                                serde_json::Value::String(s) => {
                                    encoder.append_pair(k, s);
                                }
                                serde_json::Value::Null => {
                                    encoder.append_key_only(k);
                                }
                                _ => {
                                    encoder.append_pair(k, &entry.to_string());
                                }
                            }
                        }
                    }
                    serde_json::Value::String(s) => {
                        encoder.append_key_only(s);
                    }
                    _ => {
                        encoder.append_key_only(&value.to_string());
                    }
                }
                encoder.finish()
            }
            Payload::Raw(s) => form_urlencoded::byte_serialize(s)
                .collect::<Vec<_>>()
                .join(""),
            Payload::Error(err) => {
                // FIXME what is the best behavior here?
                log::debug!("attempting to produce query from an error value: {err}");
                "".into()
            }
        }
    }

    pub fn json_null() -> Self {
        Self::Json(Json::Null)
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
        Some(p) => match p.to_bytes(None) {
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

pub fn urlencoded_bytes_to_map(input: &[u8]) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();

    for (k, v) in form_urlencoded::parse(input) {
        map.insert(k.into(), v.into());
    }

    map
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_bytes_json_string() {
        let raw = "my string";
        let encoded = "\"my string\"";

        let payload = Payload::Json(Json::String(raw.into()));

        let payload_to_string = |ct: Option<&str>| -> String {
            let bytes = payload.to_bytes(ct).expect("to_bytes() shouldn't error");
            String::from_utf8(bytes).expect("bytes should be valid UTF8")
        };

        assert_eq!(raw, payload_to_string(None));
        assert_eq!(encoded, payload_to_string(Some(JSON_CONTENT_TYPE)));
    }
}
