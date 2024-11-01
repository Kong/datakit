use proxy_wasm::traits::*;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use crate::config::get_config_value;
use crate::data::{Input, Phase, State, State::*};
use crate::nodes::{Node, NodeConfig, NodeDefaultLink, NodeFactory, PortConfig};
use crate::payload;
use crate::payload::Payload;

#[derive(Debug)]
pub struct ExitConfig {
    name: String,
    status: Option<u32>,
    warn_headers_sent: AtomicBool,
}

impl Clone for ExitConfig {
    fn clone(&self) -> ExitConfig {
        ExitConfig {
            name: self.name.clone(),
            status: self.status,
            warn_headers_sent: AtomicBool::new(self.warn_headers_sent.load(Relaxed)),
        }
    }
}

impl NodeConfig for ExitConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn default_outputs(&self) -> Option<Vec<NodeDefaultLink>> {
        Some(vec![
            NodeDefaultLink {
                this_port: "body".into(),
                other_node: "response".into(),
                other_port: "body".into(),
            },
            NodeDefaultLink {
                this_port: "headers".into(),
                other_node: "response".into(),
                other_port: "headers".into(),
            },
        ])
    }
}

#[derive(Clone)]
pub struct Exit {
    config: ExitConfig,
}

fn warn_headers_sent(config: &ExitConfig, set_headers: bool) {
    let name = &config.name;
    let set_status = config.status.is_some();

    if set_status || set_headers {
        let what = if set_headers && set_status {
            "status or headers"
        } else if set_status {
            "status"
        } else {
            "headers"
        };
        log::warn!(
            "exit: node '{name}' cannot set {what} when processing response body, \
                   headers already sent; set 'warn_headers_sent' to false \
                   to silence this warning",
        );
    }
    config.warn_headers_sent.store(false, Relaxed);
}

impl Node for Exit {
    fn run(&self, ctx: &dyn HttpContext, input: &Input) -> State {
        let config = &self.config;
        let body = input.data.first().unwrap_or(&None).as_deref();
        let headers = input.data.get(1).unwrap_or(&None).as_deref();

        let mut headers_vec = payload::to_pwm_headers(headers);

        if let Some(payload) = body {
            if let Some(content_type) = payload.content_type() {
                headers_vec.push(("Content-Type", content_type));
            }
        }

        let body_slice = match payload::to_pwm_body(body) {
            Ok(slice) => slice,
            Err(e) => return Fail(vec![Some(Payload::Error(e))]),
        };

        if input.phase == Phase::HttpResponseBody {
            if config.warn_headers_sent.load(Relaxed) {
                warn_headers_sent(config, headers.is_some());
            }

            if let Some(b) = body_slice {
                ctx.set_http_response_body(0, b.len(), &b);
            }
        } else {
            let status = config.status.unwrap_or(200);
            ctx.send_http_response(status, headers_vec, body_slice.as_deref());
        }

        Done(vec![None])
    }
}

pub struct ExitFactory {}

impl NodeFactory for ExitFactory {
    fn default_input_ports(&self) -> PortConfig {
        PortConfig {
            defaults: PortConfig::names(&["body", "headers"]),
            user_defined_ports: false,
        }
    }

    fn default_output_ports(&self) -> PortConfig {
        PortConfig {
            defaults: PortConfig::names(&["body", "headers"]),
            user_defined_ports: false,
        }
    }

    fn new_config(
        &self,
        name: &str,
        _inputs: &[String],
        _outputs: &[String],
        bt: &BTreeMap<String, Value>,
    ) -> Result<Box<dyn NodeConfig>, String> {
        Ok(Box::new(ExitConfig {
            name: name.to_string(),
            status: get_config_value(bt, "status"),
            warn_headers_sent: AtomicBool::new(
                get_config_value(bt, "warn_headers_sent").unwrap_or(true),
            ),
        }))
    }

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<ExitConfig>() {
            Some(cc) => Box::new(Exit { config: cc.clone() }),
            None => panic!("incompatible NodeConfig"),
        }
    }
}
