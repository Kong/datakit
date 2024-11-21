use lazy_static::lazy_static;
use proxy_wasm::{traits::*, types::*};
use std::rc::Rc;

mod config;
mod data;
mod debug;
mod dependency_graph;
mod nodes;
mod payload;

use crate::config::{Config, ImplicitNode};
use crate::data::{Data, Input, Phase, Phase::*, State};
use crate::debug::{Debug, RunMode};
use crate::dependency_graph::DependencyGraph;
use crate::nodes::{Node, NodeVec, PortConfig};
use crate::payload::Payload;
use crate::ImplicitNodeId::*;
use crate::ImplicitPortId::*;

// -----------------------------------------------------------------------------
// Implicit nodes
// -----------------------------------------------------------------------------

#[derive(Copy, Clone)]
enum ImplicitNodeId {
    Request = 0,
    ServiceRequest = 1,
    ServiceResponse = 2,
    Response = 3,
}

impl From<ImplicitNodeId> for usize {
    fn from(n: ImplicitNodeId) -> Self {
        n as usize
    }
}

#[derive(Copy, Clone)]
enum ImplicitPortId {
    Body = 0,
    Headers = 1,
}

impl From<ImplicitPortId> for usize {
    fn from(p: ImplicitPortId) -> Self {
        p as usize
    }
}

lazy_static! {
    static ref REQ_PORTS: Vec<String> = PortConfig::names(&["body", "headers"]);
    static ref RESP_PORTS: Vec<String> = PortConfig::names(&["body", "headers"]);
    static ref IMPLICIT_NODES: Vec<ImplicitNode> = vec![
        ImplicitNode::new("request", vec![], REQ_PORTS.clone()),
        ImplicitNode::new("service_request", REQ_PORTS.clone(), RESP_PORTS.clone()),
        ImplicitNode::new("service_response", vec![], RESP_PORTS.clone()),
        ImplicitNode::new("response", RESP_PORTS.clone(), RESP_PORTS.clone()),
    ];
}

// -----------------------------------------------------------------------------
// Root Context
// -----------------------------------------------------------------------------

struct DataKitFilterRootContext {
    config: Option<Rc<Config>>,
}

impl Context for DataKitFilterRootContext {}

impl RootContext for DataKitFilterRootContext {
    fn on_configure(&mut self, _config_size: usize) -> bool {
        match self.get_plugin_configuration() {
            Some(config_bytes) => match Config::new(config_bytes, &IMPLICIT_NODES) {
                Ok(config) => {
                    self.config = Some(Rc::new(config));
                    true
                }
                Err(err) => {
                    log::warn!("on_configure: {err}");
                    false
                }
            },
            None => {
                log::warn!("on_configure: failed getting configuration");
                false
            }
        }
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        log::debug!("DataKitFilterRootContext: create http context id: {context_id}");

        let config = self.config.clone()?;

        let nodes = config.build_nodes();
        let graph = config.get_graph();
        let debug = config.debug().then(|| Debug::new(&config));

        // FIXME: is it possible to do lifetime annotations
        // to avoid cloning every time?
        let data = Data::new(graph.clone());

        let do_request_headers = graph.has_dependents(Request.into(), Headers.into());
        let do_request_body = graph.has_dependents(Request.into(), Body.into());

        let do_service_request_headers = graph.has_provider(ServiceRequest.into(), Headers.into());
        let do_service_request_body = graph.has_provider(ServiceRequest.into(), Body.into());

        let do_service_response_headers =
            graph.has_dependents(ServiceResponse.into(), Headers.into());
        let do_service_response_body = graph.has_dependents(ServiceResponse.into(), Body.into());

        let do_response_headers = graph.has_provider(Response.into(), Headers.into());
        let do_response_body = graph.has_provider(Response.into(), Body.into());

        Some(Box::new(DataKitFilter {
            config,
            nodes,
            debug,
            data,
            failed: false,
            do_request_headers,
            do_request_body,
            do_service_request_headers,
            do_service_request_body,
            do_service_response_headers,
            do_service_response_body,
            do_response_headers,
            do_response_body,
        }))
    }
}

// -----------------------------------------------------------------------------
// Filter Context
// -----------------------------------------------------------------------------

pub struct DataKitFilter {
    config: Rc<Config>,
    nodes: NodeVec,
    data: Data,
    debug: Option<Debug>,
    failed: bool,
    do_request_headers: bool,
    do_request_body: bool,
    do_service_request_headers: bool,
    do_service_request_body: bool,
    do_service_response_headers: bool,
    do_service_response_body: bool,
    do_response_headers: bool,
    do_response_body: bool,
}

fn header_to_bool(header_value: &Option<String>) -> bool {
    match header_value {
        Some(val) => val != "off" && val != "false" && val != "0",
        None => false,
    }
}

impl DataKitFilter {
    fn debug_init(&mut self) {
        let trace_header = &self.get_http_request_header("X-DataKit-Debug-Trace");
        if header_to_bool(trace_header) {
            if let Some(ref mut debug) = self.debug {
                debug.set_tracing(true);
            }
            self.do_response_body = true;
        }
    }

    fn debug_done_headers(&mut self) {
        let ct = self.get_http_response_header("Content-Type");
        if let Some(ref mut debug) = self.debug {
            if debug.is_tracing() {
                debug.save_response_body_content_type(ct);
                self.set_http_response_header("Content-Type", Some("application/json"));
                self.set_http_response_header("Content-Length", None);
                self.set_http_response_header("Content-Encoding", None);
            }
        }
    }

    fn debug_done(&mut self) {
        if let Some(ref mut debug) = self.debug {
            if debug.is_tracing() {
                let trace = debug.get_trace();
                let bytes = trace.as_bytes();
                self.set_http_response_body(0, bytes.len(), bytes);
            }
        }
    }

    fn send_default_fail_response(&self) {
        let body = payload::to_json_error_body(
            "An unexpected error ocurred",
            self.get_property(vec!["ngx", "kong_request_id"]),
        );
        self.send_http_response(
            500,
            vec![("Content-Type", "application/json")],
            Some(&body.into_bytes()),
        );
    }

    fn set_implicit_data(&mut self, node: ImplicitNodeId, port: ImplicitPortId, payload: Payload) {
        let r = self.data.fill_port(node.into(), port.into(), payload);
        match r {
            Ok(()) => {
                if let Some(debug) = &mut self.debug {
                    let name = self.config.get_node_name(node.into());
                    if let Ok(state) = self.data.get_state(node.into()) {
                        debug.set_data(name, state);
                    }
                }
            }
            Err(e) => panic!("error setting implicit node data: {e}"),
        }
    }

    fn set_headers_data(&mut self, node: ImplicitNodeId, vec: Vec<(String, String)>) {
        let payload = payload::from_pwm_headers(vec);
        self.set_implicit_data(node, Headers, payload);
    }

    fn set_body_data(&mut self, node: ImplicitNodeId, payload: Payload) {
        self.set_implicit_data(node, Body, payload);
    }

    fn get_headers_data(&self, node: ImplicitNodeId) -> Option<&Payload> {
        self.data.fetch_port(node.into(), Headers.into())
    }

    fn get_body_data(&self, node: ImplicitNodeId) -> Option<&Payload> {
        self.data.fetch_port(node.into(), Body.into())
    }

    fn run_nodes(&mut self, phase: Phase) -> Action {
        let mut ret = Action::Continue;

        let mut debug_is_tracing = false;
        if let Some(ref mut debug) = self.debug {
            debug_is_tracing = debug.is_tracing();
        }

        let from = self.config.number_of_implicits();
        let to = self.config.node_count();

        while !self.failed {
            let mut any_ran = false;
            for i in from..to {
                let node: &dyn Node = self
                    .nodes
                    .get(i)
                    .expect("self.nodes doesn't match node_count")
                    .as_ref();
                if let Some(inputs) = self.data.get_inputs_for(i, None) {
                    any_ran = true;

                    let input = Input {
                        data: &inputs,
                        phase,
                    };
                    let state = node.run(self as &dyn HttpContext, &input);

                    if let Some(ref mut debug) = self.debug {
                        let name = self.config.get_node_name(i);
                        debug.run(name, &inputs, &state, RunMode::Run);
                    }

                    match state {
                        State::Done(_) => {}
                        State::Waiting(_) => {
                            ret = Action::Pause;
                        }
                        State::Fail(_) => {
                            self.failed = true;
                            if !debug_is_tracing {
                                self.send_default_fail_response();
                            }
                        }
                    }

                    self.data.set(i, state);
                }
            }
            if !any_ran {
                break;
            }
        }

        ret
    }

    fn set_service_request_headers(&mut self) {
        if self.do_service_request_headers {
            if let Some(payload) = self.get_headers_data(ServiceRequest) {
                let headers = payload::to_pwm_headers(Some(payload));
                self.set_http_request_headers(headers);
                self.do_service_request_headers = false;
            }
        }
    }

    fn set_content_headers(
        &self,
        node: ImplicitNodeId,
        set_header: impl Fn(&DataKitFilter, &str, Option<&str>),
    ) {
        if let Some(payload) = self.get_body_data(node) {
            if let Some(content_type) = payload.content_type() {
                set_header(self, "Content-Type", Some(content_type));
            }
            if let Some(content_length) = payload.len().map(|n| n.to_string()) {
                set_header(self, "Content-Length", Some(&content_length));
            } else {
                set_header(self, "Content-Length", Some("")); // FIXME: why doesn't None work?
            }
        }
        set_header(self, "Content-Encoding", None);
    }

    fn prep_service_request_body(&mut self) {
        if self.do_service_request_body {
            self.set_content_headers(ServiceRequest, |s, k, v| s.set_http_request_header(k, v));
        }
    }

    fn set_service_request_body(&mut self) {
        if self.do_service_request_body {
            if let Some(payload) = self.get_body_data(ServiceRequest) {
                let content_type = self.get_http_request_header("Content-Type");
                if let Ok(bytes) = payload.to_bytes(content_type.as_deref()) {
                    self.set_http_request_body(0, bytes.len(), &bytes);
                }
                self.do_service_request_body = false;
            }
        }
    }
}

impl Context for DataKitFilter {
    fn on_http_call_response(
        &mut self,
        token_id: u32,
        _nheaders: usize,
        _body_size: usize,
        _num_trailers: usize,
    ) {
        log::debug!("DataKitFilter: on http call response, id = {:?}", token_id);

        let from = self.config.number_of_implicits();
        let to = self.config.node_count();

        for i in from..to {
            let node: &dyn Node = self
                .nodes
                .get(i)
                .expect("self.nodes doesn't match node_count")
                .as_ref();
            if let Some(inputs) = self.data.get_inputs_for(i, Some(token_id)) {
                let input = Input {
                    data: &inputs,
                    phase: HttpCallResponse,
                };
                let state = node.resume(self, &input);

                if let Some(ref mut debug) = self.debug {
                    let name = self.config.get_node_name(i);
                    debug.run(name, &inputs, &state, RunMode::Resume);
                }

                self.data.set(i, state);
                break;
            }
        }

        self.run_nodes(HttpCallResponse);

        self.set_service_request_headers();
        self.prep_service_request_body();

        self.resume_http_request();
    }
}

impl HttpContext for DataKitFilter {
    fn on_http_request_headers(&mut self, _nheaders: usize, _eof: bool) -> Action {
        if self.debug.is_some() {
            self.debug_init()
        }

        if self.do_request_headers {
            let vec = self.get_http_request_headers();
            self.set_headers_data(Request, vec);
        }

        let action = self.run_nodes(HttpRequestHeaders);

        if self.get_http_request_header("Content-Length").is_none()
            && self.get_http_request_header("Transfer-Encoding").is_none()
        {
            self.set_service_request_headers();
        }

        self.prep_service_request_body();

        action
    }

    fn on_http_request_body(&mut self, body_size: usize, eof: bool) -> Action {
        if eof && self.do_request_body {
            if let Some(bytes) = self.get_http_request_body(0, body_size) {
                let content_type = self.get_http_request_header("Content-Type");
                if let Some(payload) = Payload::from_bytes(bytes, content_type.as_deref()) {
                    self.set_body_data(Request, payload);
                }
            }
        }

        let action = self.run_nodes(HttpRequestBody);

        self.set_service_request_headers();
        self.set_service_request_body();

        action
    }

    fn on_http_response_headers(&mut self, _nheaders: usize, _eof: bool) -> Action {
        if self.do_service_response_headers {
            let vec = self.get_http_response_headers();
            self.set_headers_data(ServiceResponse, vec);
        }

        let action = self.run_nodes(HttpResponseHeaders);

        if self.do_response_headers {
            if let Some(payload) = self.get_headers_data(Response) {
                let headers = payload::to_pwm_headers(Some(payload));
                self.set_http_response_headers(headers);
            }
        }

        if self.do_response_body {
            self.set_content_headers(Response, |s, k, v| s.set_http_response_header(k, v));
        }

        if self.debug.is_some() {
            self.debug_done_headers()
        }

        action
    }

    fn on_http_response_body(&mut self, body_size: usize, eof: bool) -> Action {
        if !eof {
            return Action::Pause;
        }

        if eof && self.do_service_response_body {
            if let Some(bytes) = self.get_http_response_body(0, body_size) {
                let content_type = self.get_http_response_header("Content-Type");
                if let Some(payload) = Payload::from_bytes(bytes, content_type.as_deref()) {
                    self.set_body_data(ServiceResponse, payload);
                }
            }
        }

        let action = self.run_nodes(HttpResponseBody);

        if self.do_response_body {
            if let Some(payload) = self.get_body_data(Response) {
                let content_type = self.get_http_response_header("Content-Type");
                if let Ok(bytes) = payload.to_bytes(content_type.as_deref()) {
                    self.set_http_response_body(0, bytes.len(), &bytes);
                } else {
                    self.set_http_response_body(0, 0, &[]);
                }
            } else if let Some(debug) = &self.debug {
                if let Some(bytes) = self.get_http_response_body(0, body_size) {
                    let content_type = debug.response_body_content_type();
                    if let Some(payload) = Payload::from_bytes(bytes, content_type.as_deref()) {
                        self.set_body_data(Response, payload);
                    }
                }
            }
        }

        if self.debug.is_some() {
            self.debug_done()
        }

        action
    }
}

proxy_wasm::main! {{
    nodes::register_node("implicit", Box::new(nodes::implicit::ImplicitFactory {}));
    nodes::register_node("handlebars", Box::new(nodes::handlebars::HandlebarsFactory {}));
    nodes::register_node("call", Box::new(nodes::call::CallFactory {}));
    nodes::register_node("exit", Box::new(nodes::exit::ExitFactory {}));
    nodes::register_node("jq", Box::new(nodes::jq::JqFactory {}));
    nodes::register_node("property", Box::new(nodes::property::PropertyFactory {}));

    proxy_wasm::set_log_level(LogLevel::Debug);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(DataKitFilterRootContext {
            config: None,
        })
    });
}}

// interesting tests to try out:
// multiple callouts at once with different settings: http 1.0, 1.1, chunked encoding, content-length

// test with bad responses
