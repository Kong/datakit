use log;
use proxy_wasm::traits::*;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;

use crate::config::get_config_value;
use crate::data::{Input, State, State::*};
use crate::nodes::{Node, NodeConfig, NodeFactory, PortConfig};
use crate::payload::Payload;

#[derive(Clone, Debug)]
pub struct PropertyConfig {
    path: Vec<String>,
    content_type: Option<String>,
}

impl PropertyConfig {
    fn new(name: String, ct: Option<String>) -> Self {
        Self {
            path: name.split('.').map(|s| s.to_string()).collect(),
            content_type: ct,
        }
    }

    fn to_path(&self) -> Vec<&str> {
        self.path.iter().map(String::as_str).collect()
    }
}

impl NodeConfig for PropertyConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Property {
    config: PropertyConfig,
}

impl Node for Property {
    fn run(&self, ctx: &dyn HttpContext, input: &Input) -> State {
        // set the property first if we have an input
        if let Some(Some(value)) = input.data.first() {
            log::debug!("SET property {:?} => {:?}", self.config.path, value);
            match (*value).clone().to_bytes() {
                Ok(bytes) => ctx.set_property(self.config.to_path(), Some(bytes.as_slice())),
                Err(e) => {
                    return Fail(vec![Some(Payload::Error(e))]);
                }
            }
        };

        Done(match ctx.get_property(self.config.to_path()) {
            Some(bytes) => {
                let payload = Payload::from_bytes(bytes, self.config.content_type.as_deref());
                log::debug!("GET property {:?} => {:?}", &self.config.path, payload);
                vec![payload]
            }
            None => {
                log::debug!("GET property {:?} => None", &self.config.path);
                vec![Some(Payload::json_null())]
            }
        })
    }
}

pub struct PropertyFactory {}

impl NodeFactory for PropertyFactory {
    fn default_input_ports(&self) -> PortConfig {
        PortConfig {
            defaults: PortConfig::names(&["value"]),
            user_defined_ports: false,
        }
    }
    fn default_output_ports(&self) -> PortConfig {
        PortConfig {
            defaults: PortConfig::names(&["value"]),
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
        Ok(Box::new(PropertyConfig::new(
            get_config_value(bt, "property")
                .ok_or_else(|| "Missing `property` attribute".to_owned())?,
            get_config_value(bt, "content_type"),
        )))
    }

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<PropertyConfig>() {
            Some(cc) => Box::new(Property { config: cc.clone() }),
            None => panic!("incompatible NodeConfig"),
        }
    }
}

///* TODO: see if we can use https://github.com/proxy-wasm/test-framework
#[cfg(test)]
mod test {
    use proxy_wasm::types::Bytes;
    use std::{cell::RefCell, collections::HashMap};

    use super::*;

    #[derive(Debug, Clone, Default)]
    struct Mock {
        // Context::{get_property,set_property}() methods receive an immutable
        // reference to $self, so use RefCell for interior mutability
        props: RefCell<HashMap<Vec<String>, Vec<u8>>>,
    }

    impl Mock {
        fn new() -> Self {
            Default::default()
        }

        fn set(&self, name: &str, value: &str) {
            let path = to_path(name.split(".").collect());
            let bytes = value.bytes().collect();
            self.props.borrow_mut().insert(path, bytes);
        }

        fn get(&self, name: &str) -> Option<String> {
            let path = to_path(name.split(".").collect());
            self.props
                .borrow()
                .get(&path)
                .cloned()
                .map(|value| String::from_utf8(value).unwrap())
        }
    }

    fn to_path(path: Vec<&str>) -> Vec<String> {
        path.iter().map(|s| s.to_string()).collect()
    }

    impl Context for Mock {
        fn get_property(&self, path: Vec<&str>) -> Option<Bytes> {
            self.props.borrow().get(&to_path(path)).cloned()
        }

        fn set_property(&self, path: Vec<&str>, value: Option<&[u8]>) {
            let path = to_path(path);
            match value {
                Some(bytes) => self.props.borrow_mut().insert(path, bytes.into()),
                None => self.props.borrow_mut().remove(&path),
            };
        }

        fn get_current_time(&self) -> std::time::SystemTime {
            todo!("mock function")
        }

        fn get_shared_data(&self, _key: &str) -> (Option<Bytes>, Option<u32>) {
            todo!("mock function")
        }

        fn set_shared_data(
            &self,
            _key: &str,
            _value: Option<&[u8]>,
            _cas: Option<u32>,
        ) -> Result<(), proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn register_shared_queue(&self, _name: &str) -> u32 {
            todo!("mock function")
        }

        fn resolve_shared_queue(&self, _vm_id: &str, _name: &str) -> Option<u32> {
            todo!("mock function")
        }

        fn dequeue_shared_queue(
            &self,
            _queue_id: u32,
        ) -> Result<Option<Bytes>, proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn enqueue_shared_queue(
            &self,
            _queue_id: u32,
            _value: Option<&[u8]>,
        ) -> Result<(), proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn dispatch_http_call(
            &self,
            _upstream: &str,
            _headers: Vec<(&str, &str)>,
            _body: Option<&[u8]>,
            _trailers: Vec<(&str, &str)>,
            _timeout: std::time::Duration,
        ) -> Result<u32, proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn on_http_call_response(
            &mut self,
            _token_id: u32,
            _num_headers: usize,
            _body_size: usize,
            _num_trailers: usize,
        ) {
        }

        fn get_http_call_response_headers(&self) -> Vec<(String, String)> {
            todo!("mock function")
        }

        fn get_http_call_response_headers_bytes(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn get_http_call_response_header(&self, _name: &str) -> Option<String> {
            todo!("mock function")
        }

        fn get_http_call_response_header_bytes(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn get_http_call_response_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
            todo!("mock function")
        }

        fn get_http_call_response_trailers(&self) -> Vec<(String, String)> {
            todo!("mock function")
        }

        fn get_http_call_response_trailers_bytes(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn get_http_call_response_trailer(&self, _name: &str) -> Option<String> {
            todo!("mock function")
        }

        fn get_http_call_response_trailer_bytes(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn dispatch_grpc_call(
            &self,
            _upstream_name: &str,
            _service_name: &str,
            _method_name: &str,
            _initial_metadata: Vec<(&str, &[u8])>,
            _message: Option<&[u8]>,
            _timeout: std::time::Duration,
        ) -> Result<u32, proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn on_grpc_call_response(
            &mut self,
            _token_id: u32,
            _status_code: u32,
            _response_size: usize,
        ) {
        }

        fn get_grpc_call_response_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
            todo!("mock function")
        }

        fn cancel_grpc_call(&self, _token_id: u32) {
            todo!("mock function")
        }

        fn open_grpc_stream(
            &self,
            _cluster_name: &str,
            _service_name: &str,
            _method_name: &str,
            _initial_metadata: Vec<(&str, &[u8])>,
        ) -> Result<u32, proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn on_grpc_stream_initial_metadata(&mut self, _token_id: u32, _num_elements: u32) {}

        fn get_grpc_stream_initial_metadata(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn get_grpc_stream_initial_metadata_value(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn send_grpc_stream_message(
            &self,
            _token_id: u32,
            _message: Option<&[u8]>,
            _end_stream: bool,
        ) {
            todo!("mock function")
        }

        fn on_grpc_stream_message(&mut self, _token_id: u32, _message_size: usize) {}

        fn get_grpc_stream_message(&mut self, _start: usize, _max_size: usize) -> Option<Bytes> {
            todo!("mock function")
        }

        fn on_grpc_stream_trailing_metadata(&mut self, _token_id: u32, _num_elements: u32) {}

        fn get_grpc_stream_trailing_metadata(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn get_grpc_stream_trailing_metadata_value(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn cancel_grpc_stream(&self, _token_id: u32) {
            todo!("mock function")
        }

        fn close_grpc_stream(&self, _token_id: u32) {
            todo!("mock function")
        }

        fn on_grpc_stream_close(&mut self, _token_id: u32, _status_code: u32) {}

        fn get_grpc_status(&self) -> (u32, Option<String>) {
            todo!("mock function")
        }

        fn call_foreign_function(
            &self,
            _function_name: &str,
            _arguments: Option<&[u8]>,
        ) -> Result<Option<Bytes>, proxy_wasm::types::Status> {
            todo!("mock function")
        }

        fn on_done(&mut self) -> bool {
            true
        }

        fn done(&self) {
            todo!("mock function")
        }
    }

    impl HttpContext for Mock {
        fn on_http_request_headers(
            &mut self,
            _num_headers: usize,
            _end_of_stream: bool,
        ) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }

        fn get_http_request_headers(&self) -> Vec<(String, String)> {
            todo!("mock function")
        }

        fn get_http_request_headers_bytes(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn set_http_request_headers(&self, _headers: Vec<(&str, &str)>) {
            todo!("mock function")
        }

        fn set_http_request_headers_bytes(&self, _headers: Vec<(&str, &[u8])>) {
            todo!("mock function")
        }

        fn get_http_request_header(&self, _name: &str) -> Option<String> {
            todo!("mock function")
        }

        fn get_http_request_header_bytes(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn set_http_request_header(&self, _name: &str, _value: Option<&str>) {
            todo!("mock function")
        }

        fn set_http_request_header_bytes(&self, _name: &str, _value: Option<&[u8]>) {
            todo!("mock function")
        }

        fn add_http_request_header(&self, _name: &str, _value: &str) {
            todo!("mock function")
        }

        fn add_http_request_header_bytes(&self, _name: &str, _value: &[u8]) {
            todo!("mock function")
        }

        fn on_http_request_body(
            &mut self,
            _body_size: usize,
            _end_of_stream: bool,
        ) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }

        fn get_http_request_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
            todo!("mock function")
        }

        fn set_http_request_body(&self, _start: usize, _size: usize, _value: &[u8]) {
            todo!("mock function")
        }

        fn on_http_request_trailers(&mut self, _num_trailers: usize) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }

        fn get_http_request_trailers(&self) -> Vec<(String, String)> {
            todo!("mock function")
        }

        fn get_http_request_trailers_bytes(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn set_http_request_trailers(&self, _trailers: Vec<(&str, &str)>) {
            todo!("mock function")
        }

        fn set_http_request_trailers_bytes(&self, _trailers: Vec<(&str, &[u8])>) {
            todo!("mock function")
        }

        fn get_http_request_trailer(&self, _name: &str) -> Option<String> {
            todo!("mock function")
        }

        fn get_http_request_trailer_bytes(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn set_http_request_trailer(&self, _name: &str, _value: Option<&str>) {
            todo!("mock function")
        }

        fn set_http_request_trailer_bytes(&self, _name: &str, _value: Option<&[u8]>) {
            todo!("mock function")
        }

        fn add_http_request_trailer(&self, _name: &str, _value: &str) {
            todo!("mock function")
        }

        fn add_http_request_trailer_bytes(&self, _name: &str, _value: &[u8]) {
            todo!("mock function")
        }

        fn resume_http_request(&self) {
            todo!("mock function")
        }

        fn reset_http_request(&self) {
            todo!("mock function")
        }

        fn on_http_response_headers(
            &mut self,
            _num_headers: usize,
            _end_of_stream: bool,
        ) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }

        fn get_http_response_headers(&self) -> Vec<(String, String)> {
            todo!("mock function")
        }

        fn get_http_response_headers_bytes(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn set_http_response_headers(&self, _headers: Vec<(&str, &str)>) {
            todo!("mock function")
        }

        fn set_http_response_headers_bytes(&self, _headers: Vec<(&str, &[u8])>) {
            todo!("mock function")
        }

        fn get_http_response_header(&self, _name: &str) -> Option<String> {
            todo!("mock function")
        }

        fn get_http_response_header_bytes(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn set_http_response_header(&self, _name: &str, _value: Option<&str>) {
            todo!("mock function")
        }

        fn set_http_response_header_bytes(&self, _name: &str, _value: Option<&[u8]>) {
            todo!("mock function")
        }

        fn add_http_response_header(&self, _name: &str, _value: &str) {
            todo!("mock function")
        }

        fn add_http_response_header_bytes(&self, _name: &str, _value: &[u8]) {
            todo!("mock function")
        }

        fn on_http_response_body(
            &mut self,
            _body_size: usize,
            _end_of_stream: bool,
        ) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }

        fn get_http_response_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
            todo!("mock function")
        }

        fn set_http_response_body(&self, _start: usize, _size: usize, _value: &[u8]) {
            todo!("mock function")
        }

        fn on_http_response_trailers(&mut self, _num_trailers: usize) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }

        fn get_http_response_trailers(&self) -> Vec<(String, String)> {
            todo!("mock function")
        }

        fn get_http_response_trailers_bytes(&self) -> Vec<(String, Bytes)> {
            todo!("mock function")
        }

        fn set_http_response_trailers(&self, _trailers: Vec<(&str, &str)>) {
            todo!("mock function")
        }

        fn set_http_response_trailers_bytes(&self, _trailers: Vec<(&str, &[u8])>) {
            todo!("mock function")
        }

        fn get_http_response_trailer(&self, _name: &str) -> Option<String> {
            todo!("mock function")
        }

        fn get_http_response_trailer_bytes(&self, _name: &str) -> Option<Bytes> {
            todo!("mock function")
        }

        fn set_http_response_trailer(&self, _name: &str, _value: Option<&str>) {
            todo!("mock function")
        }

        fn set_http_response_trailer_bytes(&self, _name: &str, _value: Option<&[u8]>) {
            todo!("mock function")
        }

        fn add_http_response_trailer(&self, _name: &str, _value: &str) {
            todo!("mock function")
        }

        fn add_http_response_trailer_bytes(&self, _name: &str, _value: &[u8]) {
            todo!("mock function")
        }

        fn resume_http_response(&self) {
            todo!("mock function")
        }

        fn reset_http_response(&self) {
            todo!("mock function")
        }

        fn send_http_response(
            &self,
            _status_code: u32,
            _headers: Vec<(&str, &str)>,
            _body: Option<&[u8]>,
        ) {
            todo!("mock function")
        }

        fn send_grpc_response(
            &self,
            _grpc_status: proxy_wasm::types::GrpcStatusCode,
            _grpc_status_message: Option<&str>,
            _custom_metadata: Vec<(&str, &[u8])>,
        ) {
            todo!("mock function")
        }

        fn on_log(&mut self) {}
    }

    #[test]
    fn get_property() {
        let input = Input {
            data: &[],
            phase: crate::data::Phase::HttpRequestHeaders,
        };

        let ctx = Mock::new();
        let value = "test.value";
        ctx.set("test.property", value);

        let prop = Property {
            config: PropertyConfig::new("test.property".into(), None),
        };

        let state = Node::run(&prop, &ctx as &dyn HttpContext, &input);
        assert_eq!(State::Done(vec![Some(Payload::Raw(value.into()))]), state);
    }

    #[test]
    fn set_property() {
        let property = "test.property";
        let value = "test.value";
        let payload = Payload::Raw(value.into());

        let input = Input {
            data: &[Some(&payload)],
            phase: crate::data::Phase::HttpRequestHeaders,
        };

        let ctx = Mock::new();
        assert_eq!(None, ctx.get(property));

        let prop = Property {
            config: PropertyConfig::new(property.into(), None),
        };

        let state = Node::run(&prop, &ctx as &dyn HttpContext, &input);
        assert_eq!(State::Done(vec![Some(Payload::Raw(value.into()))]), state);

        assert_eq!(Some(value.into()), ctx.get(property));
    }
}
