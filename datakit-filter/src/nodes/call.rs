use log;
use proxy_wasm::{traits::*, types::*};
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::time::Duration;
use url::Url;

use crate::data;
use crate::data::{Payload, State, State::*};
use crate::nodes::Connections;
use crate::nodes::{get_config_value, Node, NodeConfig};

#[derive(Deserialize, Clone, Debug)]
pub struct CallConfig {
    connections: Connections,

    // FIXME: the optional ones should be Option,
    // but we're not really serializing this for now, just deserializing...

    // node-specific configuration fields:
    url: String,
    method: String,
    timeout: u32,
}

impl NodeConfig for CallConfig {
    fn get_connections(&self) -> &Connections {
        &self.connections
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_dyn(&self) -> Box<dyn NodeConfig> {
        Box::new(self.clone())
    }

    fn get_node_type(&self) -> &'static str {
        "call"
    }

    fn from_map(bt: BTreeMap<String, Value>, connections: Connections) -> Box<dyn NodeConfig>
    where
        Self: Sized,
    {
        Box::new(CallConfig {
            connections,
            url: get_config_value(&bt, "url", String::from("")),
            method: get_config_value(&bt, "method", String::from("GET")),
            timeout: get_config_value(&bt, "timeout", 60),
        })
    }
}

#[derive(Clone)]
pub struct Call {
    config: CallConfig,

    token_id: Option<u32>,
}

impl Call {
    fn dispatch_call(
        &self,
        ctx: &dyn HttpContext,
        body: Option<&Payload>,
        headers: Option<&Payload>,
    ) -> Result<u32, Status> {
        log::debug!(
            "call: {} - url: {}",
            self.config.connections.name,
            self.config.url
        );

        let call_url = Url::parse(self.config.url.as_str()).map_err(|r| {
            log::error!("call: failed parsing URL from 'url' field: {}", r);
            Status::BadArgument
        })?;

        let host = match call_url.host_str() {
            Some(h) => Ok(h),
            None => {
                log::error!("call: failed getting host from URL");
                Err(Status::BadArgument)
            }
        }?;

        let mut headers_vec = data::to_pwm_headers(headers);
        headers_vec.push((":method", self.config.method.as_str()));
        headers_vec.push((":path", call_url.path()));

        let body_slice = data::to_pwm_body(body);

        let trailers = vec![];
        let timeout = Duration::from_secs(self.config.timeout.into());

        let sch_host_port = match call_url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_owned(),
        };
        ctx.dispatch_http_call(
            &sch_host_port,
            headers_vec,
            body_slice.as_deref(),
            trailers,
            timeout,
        )
    }
}

impl Node for Call {
    fn new_box(config: &Box<dyn NodeConfig>) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<CallConfig>() {
            Some(cc) => Box::new(Call {
                config: cc.clone(),
                token_id: None,
            }),
            None => panic!("incompatible NodeConfig"),
        }
    }

    fn get_name(&self) -> &str {
        &self.config.connections.name
    }

    fn run(&mut self, ctx: &dyn HttpContext, inputs: Vec<Option<&Payload>>) -> State {
        log::debug!("call: run");

        let body = inputs.first().unwrap_or(&None);
        let headers = inputs.get(1).unwrap_or(&None);

        match self.dispatch_call(ctx, *body, *headers) {
            Ok(id) => {
                log::debug!("call: dispatch call id: {:?}", id);
                self.token_id = Some(id);
                return Waiting(id);
            }
            Err(status) => {
                log::error!("call: error: {:?}", status);
            }
        }

        Done(None)
    }

    fn resume(&mut self, ctx: &dyn HttpContext, _inputs: Vec<Option<&Payload>>) -> State {
        log::debug!("call: resume");

        let r = if let Some(body) = ctx.get_http_call_response_body(0, usize::MAX) {
            Payload::from_bytes(
                body,
                ctx.get_http_call_response_header("Content-Type").as_deref(),
            )
        } else {
            None
        };

        Done(r)
    }

    fn is_waiting_on(&self, token_id: u32) -> bool {
        self.token_id == Some(token_id)
    }
}