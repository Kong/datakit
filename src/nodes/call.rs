use log;
use proxy_wasm::traits::*;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::time::Duration;
use url::Url;

use crate::config::get_config_value;
use crate::data::{Input, State, State::*};
use crate::nodes::{Node, NodeConfig, NodeFactory, PortConfig};
use crate::payload;
use crate::payload::Payload;

#[derive(Clone, Debug)]
pub struct CallConfig {
    // FIXME: the optional ones should be Option,
    // but we're not really serializing this for now, just deserializing...

    // node-specific configuration fields:
    url: String,
    method: String,
    timeout: u32,
}

impl NodeConfig for CallConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Call {
    config: CallConfig,
}

impl Node for Call {
    fn run(&self, ctx: &dyn HttpContext, input: &Input) -> State {
        log::debug!("call: run");

        let body = input.data.first().unwrap_or(&None);
        let headers = input.data.get(1).unwrap_or(&None);

        let call_url = match Url::parse(self.config.url.as_str()) {
            Ok(u) => u,
            Err(err) => {
                log::error!("call: failed parsing URL from 'url' field: {err}");
                return Done(vec![]);
            }
        };

        let host = match call_url.host_str() {
            Some(h) => h,
            None => {
                log::error!("call: failed getting host from URL");
                return Done(vec![]);
            }
        };

        let body_slice = match payload::to_pwm_body(*body) {
            Ok(slice) => slice,
            Err(e) => return Fail(vec![Some(Payload::Error(e))]),
        };

        let trailers = vec![];
        let timeout = Duration::from_secs(self.config.timeout.into());

        let host_port = match call_url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        };

        let mut headers_vec = payload::to_pwm_headers(*headers);
        headers_vec.push((":method", self.config.method.as_str()));
        headers_vec.push((":path", call_url.path()));
        headers_vec.push((":scheme", call_url.scheme()));
        headers_vec.push((":authority", &host_port));

        let result = ctx.dispatch_http_call(
            &host_port,
            headers_vec,
            body_slice.as_deref(),
            trailers,
            timeout,
        );

        match result {
            Ok(id) => {
                log::debug!("call: dispatch call id: {:?}", id);
                Waiting(id)
            }
            Err(status) => {
                log::debug!("call: dispatch call failed: {:?}", status);
                Fail(vec![Some(Payload::Error(format!("error: {:?}", status)))])
            }
        }
    }

    fn resume(&self, ctx: &dyn HttpContext, _inputs: &Input) -> State {
        log::debug!("call: resume");

        let headers = Some(payload::from_pwm_headers(
            ctx.get_http_call_response_headers(),
        ));

        let body = if let Some(body) = ctx.get_http_call_response_body(0, usize::MAX) {
            let content_type = ctx.get_http_call_response_header("Content-Type");

            Payload::from_bytes(body, content_type.as_deref())
        } else {
            None
        };

        // TODO only produce an output if it is connected
        // TODO produce a Fail() status on HTTP >= 400

        Done(vec![body, headers])
    }
}

pub struct CallFactory {}

impl NodeFactory for CallFactory {
    fn default_input_ports(&self) -> PortConfig {
        PortConfig {
            defaults: PortConfig::names(&["body", "headers", "query"]),
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
        _name: &str,
        _inputs: &[String],
        _outputs: &[String],
        bt: &BTreeMap<String, Value>,
    ) -> Result<Box<dyn NodeConfig>, String> {
        Ok(Box::new(CallConfig {
            url: get_config_value(bt, "url").unwrap_or_else(|| String::from("")),
            method: get_config_value(bt, "method").unwrap_or_else(|| String::from("GET")),
            timeout: get_config_value(bt, "timeout").unwrap_or(60),
        }))
    }

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<CallConfig>() {
            Some(cc) => Box::new(Call { config: cc.clone() }),
            None => panic!("incompatible NodeConfig"),
        }
    }
}
