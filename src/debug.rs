use crate::config::Config;
use crate::data::State;
use crate::payload::Payload;

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub enum RunMode {
    Run,
    Resume,
}

pub enum DataMode {
    Done,
    Waiting,
    Fail,
}

struct RunOperation {
    node_name: String,
    node_type: String,
    action: RunMode,
    at: Option<Duration>,
    duration: Option<Duration>,
}

#[derive(Serialize)]
struct PortValue {
    data_type: String,
    value: Option<Value>,
}

struct SetOperation {
    node_name: String,
    status: DataMode,
    values: Vec<PortValue>,
    at: Option<Duration>,
}

enum Operation {
    Run(RunOperation),
    Set(SetOperation),
}

pub struct Debug {
    trace: bool,
    operations: Vec<Operation>,
    node_types: HashMap<String, String>,
    orig_response_body_content_type: Option<String>,
    start_time: SystemTime,
    node_starts: HashMap<String, SystemTime>,
}

impl State {
    fn to_data_mode(&self) -> DataMode {
        match self {
            State::Done(_) => DataMode::Done,
            State::Waiting(_) => DataMode::Waiting,
            State::Fail(_) => DataMode::Fail,
        }
    }
}

fn payloads_to_values(payloads: &[Option<Payload>], default_type: &str) -> Vec<PortValue> {
    payloads
        .iter()
        .map(|p| match p {
            Some(payload) => match payload.to_json() {
                Ok(v) => PortValue {
                    data_type: payload.content_type().unwrap_or(default_type).to_string(),
                    value: Some(v),
                },
                Err(e) => PortValue {
                    data_type: "fail".into(),
                    value: Some(serde_json::json!(e)),
                },
            },
            None => PortValue {
                data_type: "none".into(),
                value: None,
            },
        })
        .collect()
}

impl Debug {
    pub fn new(config: &Config) -> Debug {
        let mut node_types = HashMap::new();
        for (name, node_type) in config.node_types() {
            node_types.insert(name.to_string(), node_type.to_string());
        }

        Debug {
            node_types,
            trace: false,
            operations: vec![],
            orig_response_body_content_type: None,
            start_time: SystemTime::now(),
            node_starts: HashMap::new(),
        }
    }

    pub fn set_data(&mut self, name: &str, state: &State) {
        if self.trace {
            self.operations.push(Operation::Set(SetOperation {
                node_name: name.to_string(),
                status: state.to_data_mode(),
                values: match state {
                    State::Waiting(_) => vec![],
                    State::Done(p) => payloads_to_values(p, "raw"),
                    State::Fail(p) => payloads_to_values(p, "fail"),
                },
                at: Some(self.start_time.elapsed().unwrap()),
            }));
        }
    }

    pub fn run(&mut self, name: &str, _args: &[Option<&Payload>], state: &State, action: RunMode) {
        if self.trace {
            let node_type = self.node_types.get(name).expect("node exists");

            let mut at = None;
            let mut duration = None;
            match action {
                RunMode::Run => {
                    self.node_starts.insert(name.into(), SystemTime::now());
                    at = Some(self.start_time.elapsed().unwrap());
                }
                RunMode::Resume => {
                    duration = Some(self.node_starts.get(name).unwrap().elapsed().unwrap());
                }
            }

            self.operations.push(Operation::Run(RunOperation {
                action,
                node_name: name.to_string(),
                node_type: node_type.to_string(),
                at,
                duration,
            }));

            self.set_data(name, state);
        }
    }

    pub fn save_response_body_content_type(&mut self, ct: Option<String>) {
        self.orig_response_body_content_type = ct;
    }

    pub fn response_body_content_type(&self) -> &Option<String> {
        &self.orig_response_body_content_type
    }

    pub fn set_tracing(&mut self, enable: bool) {
        self.trace = enable;
    }

    pub fn is_tracing(&self) -> bool {
        self.trace
    }

    pub fn get_trace(&self) -> String {
        #[derive(Serialize)]
        struct TraceAction<'a> {
            action: &'static str,
            name: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            r#type: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            values: Option<&'a Vec<PortValue>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            at: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            duration: Option<f32>,
        }

        let mut actions: Vec<TraceAction> = vec![];

        for op in self.operations.iter() {
            actions.push(match op {
                Operation::Run(run) => TraceAction {
                    action: match run.action {
                        RunMode::Run => "run",
                        RunMode::Resume => "resume",
                    },
                    r#type: Some(&run.node_type),
                    name: &run.node_name,
                    values: None,
                    at: run.at.map(|d| d.as_secs_f32()),
                    duration: run.duration.map(|d| d.as_secs_f32()),
                },
                Operation::Set(set) => match set.status {
                    DataMode::Done => TraceAction {
                        action: "value",
                        name: &set.node_name,
                        r#type: None,
                        values: Some(&set.values),
                        at: set.at.map(|d| d.as_secs_f32()),
                        duration: None,
                    },
                    DataMode::Waiting => TraceAction {
                        action: "wait",
                        name: &set.node_name,
                        r#type: None,
                        values: None,
                        at: set.at.map(|d| d.as_secs_f32()),
                        duration: None,
                    },
                    DataMode::Fail => TraceAction {
                        action: "fail",
                        name: &set.node_name,
                        r#type: None,
                        values: Some(&set.values),
                        at: set.at.map(|d| d.as_secs_f32()),
                        duration: None,
                    },
                },
            });
        }

        serde_json::json!(actions).to_string()
    }
}
