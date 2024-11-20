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
    fn new<T, CT>(name: T, ct: Option<CT>) -> Self
    where
        T: AsRef<str>,
        Option<CT>: Into<Option<String>>,
    {
        Self {
            path: name.as_ref().split('.').map(|s| s.to_string()).collect(),
            content_type: ct.into(),
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

#[derive(Debug)]
pub struct Property {
    config: PropertyConfig,
}

impl From<PropertyConfig> for Property {
    fn from(value: PropertyConfig) -> Self {
        Self { config: value }
    }
}

impl From<&PropertyConfig> for Property {
    fn from(value: &PropertyConfig) -> Self {
        Self::from(value.clone())
    }
}

impl Property {
    fn set(&self, ctx: &dyn HttpContext, payload: &Payload) -> State {
        #[cfg(debug_assertions)]
        log::debug!("SET property {:?} => {:?}", self.config.path, payload);

        let content_type = self.config.content_type.as_deref();

        match payload.to_bytes(content_type) {
            Ok(bytes) => {
                ctx.set_property(self.config.to_path(), Some(bytes.as_slice()));
                // XXX: we have to return _something_ here or else things
                // blow up, but passing the input payload through would
                // require a clone
                Done(vec![Some(Payload::json_null())])
            }
            Err(e) => Fail(vec![Some(Payload::Error(e))]),
        }
    }

    fn get(&self, ctx: &dyn HttpContext) -> State {
        let content_type = self.config.content_type.as_deref();

        Done(match ctx.get_property(self.config.to_path()) {
            Some(bytes) => {
                let payload = Payload::from_bytes(bytes, content_type);

                #[cfg(debug_assertions)]
                log::debug!("GET property {:?} => {:?}", &self.config.path, payload);

                vec![payload]
            }
            None => {
                #[cfg(debug_assertions)]
                log::debug!("GET property {:?} => None", &self.config.path);

                vec![Some(Payload::json_null())]
            }
        })
    }
}

impl Node for Property {
    fn run(&self, ctx: &dyn HttpContext, input: &Input) -> State {
        // set the property if we have an input
        if let Some(Some(payload)) = input.data.first() {
            return self.set(ctx, payload);
        }

        self.get(ctx)
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
            get_config_value::<String>(bt, "property")
                .ok_or_else(|| "Missing `property` attribute".to_owned())?,
            get_config_value::<String>(bt, "content_type"),
        )))
    }

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<PropertyConfig>() {
            Some(cc) => Box::new(Property::from(cc)),
            None => panic!("incompatible NodeConfig"),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::payload::JSON_CONTENT_TYPE;
    use mock_proxy_wasm::*;
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

    #[mock_proxy_wasm_context]
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
    }

    #[mock_proxy_wasm_http_context]
    impl HttpContext for Mock {}

    macro_rules! input {
        ($v:expr) => {
            Input {
                data: &[$v],
                phase: crate::data::Phase::HttpRequestHeaders,
            }
        };
        () => {
            Input {
                data: &[],
                phase: crate::data::Phase::HttpRequestHeaders,
            }
        };
    }

    macro_rules! done {
        ($v:expr) => {
            State::Done(vec![$v])
        };
        () => {
            State::Done(vec![Some(Payload::json_null())])
        };
    }

    macro_rules! fail {
        ($v:expr) => {
            State::Fail(vec![$v])
        };
    }

    macro_rules! run {
        ($node:expr, $ctx:expr, $input:expr) => {
            Node::run($node, $ctx as &dyn HttpContext, $input)
        };
    }

    macro_rules! node {
        ($name:expr) => {
            Property::from(PropertyConfig::new($name, None as Option<String>))
        };
        ($name:expr, $ct:expr) => {
            Property::from(PropertyConfig::new($name, Some($ct.into())))
        };
    }

    #[test]
    fn get_property() {
        let property = "test.property";
        let value = "test.value";

        let ctx = Mock::new();
        ctx.set(property, value);

        let node = node!(property);
        let input = input!();

        let state = run!(&node, &ctx, &input);
        assert_eq!(done!(Some(Payload::Raw(value.into()))), state);
    }

    #[test]
    fn get_property_not_exists() {
        let ctx = Mock::new();

        let node = node!("test.property");

        let state = run!(&node, &ctx, &input!());
        assert_eq!(done!(Some(Payload::json_null())), state);
    }

    #[test]
    fn get_property_json() {
        let property = "test.property";
        let value = r#"{ "a": 1 }"#;

        let payload =
            Payload::from_bytes(value.into(), Some(JSON_CONTENT_TYPE)).expect("unreachable");

        let ctx = Mock::new();
        ctx.set(property, value);

        let node = node!(property, JSON_CONTENT_TYPE);

        let state = run!(&node, &ctx, &input!());
        assert_eq!(done!(Some(payload)), state);
    }

    #[test]
    fn get_property_json_invalid() {
        let property = "test.property";
        let value = r#"{ "a": }"#;

        let ctx = Mock::new();
        ctx.set(property, value);

        let node = node!(property, JSON_CONTENT_TYPE);

        let state = run!(&node, &ctx, &input!());
        let State::Done(payloads) = state else {
            panic!("expected State::Done(...)");
        };

        assert_eq!(1, payloads.len());

        let Some(&Some(Payload::Error(_))) = payloads.first() else {
            panic!("expected Payload::Error(...)");
        };
    }

    #[test]
    fn set_property() {
        let property = "test.property";
        let value = "test.value";
        let ctx = Mock::new();

        let node = node!(property);
        let payload = Payload::Raw(value.into());
        let input = input!(Some(&payload));

        let state = run!(&node, &ctx, &input);
        assert_eq!(done!(), state);
        assert_eq!(Some(value.into()), ctx.get(property));
    }

    #[test]
    fn set_property_from_json() {
        let property = "test.property";

        let ctx = Mock::new();

        let json = serde_json::json!({
            "a": 1,
        });

        let payload = Payload::Json(json.clone());

        let node = node!(property);
        let state = run!(&node, &ctx, &input!(Some(&payload)));
        assert_eq!(done!(), state);
    }

    #[test]
    fn set_property_from_json_invalid() {
        let property = "test.property";
        let value = r#"{ "a": }"#;

        let ctx = Mock::new();
        ctx.set(property, value);

        let node = node!(property, JSON_CONTENT_TYPE);

        let state = run!(&node, &ctx, &input!());
        let State::Done(payloads) = state else {
            panic!("expected State::Done(...)");
        };

        assert_eq!(1, payloads.len());

        let Some(&Some(Payload::Error(_))) = payloads.first() else {
            panic!("expected Payload::Error(...)");
        };
    }

    #[test]
    fn set_property_from_json_plain() {
        let property = "test.property";

        let ctx = Mock::new();

        let json = serde_json::json!({
            "a": 1,
        });

        let encoded = json.to_string();

        let json = Payload::Json(json);

        let node = node!(property, "text/plain");
        let state = run!(&node, &ctx, &input!(Some(&json)));
        assert_eq!(done!(), state);
        assert_eq!(Some(encoded), ctx.get(property));
    }

    #[test]
    fn set_property_from_json_string_no_content_type() {
        let property = "test.property";

        let ctx = Mock::new();

        let raw = "my string".to_string();
        let json = serde_json::Value::String(raw.clone());

        let payload = Payload::Json(json.clone());

        let node = node!(property);
        let state = run!(&node, &ctx, &input!(Some(&payload)));

        assert_eq!(done!(), state);
        assert_eq!(Some(raw), ctx.get(property));
    }

    #[test]
    fn set_property_from_json_string_plain_content_type() {
        let property = "test.property";

        let ctx = Mock::new();

        let raw = "my string".to_string();
        let json = serde_json::Value::String(raw.clone());

        let payload = Payload::Json(json.clone());

        let node = node!(property, "text/plain");
        let state = run!(&node, &ctx, &input!(Some(&payload)));

        assert_eq!(done!(), state);
        assert_eq!(Some(raw), ctx.get(property));
    }

    #[test]
    fn set_property_from_json_string_json_content_type() {
        let property = "test.property";

        let ctx = Mock::new();

        let raw = "my string".to_string();
        let json = serde_json::Value::String(raw.clone());
        let encoded = json.to_string();

        let payload = Payload::Json(json.clone());

        let node = node!(property, JSON_CONTENT_TYPE);
        let state = run!(&node, &ctx, &input!(Some(&payload)));

        assert_eq!(done!(), state);
        assert_eq!(Some(encoded), ctx.get(property));
    }

    #[test]
    fn update_property() {
        let property = "test.property";
        let old = "old value";
        let new = "new value";

        let ctx = Mock::new();
        ctx.set(property, old);

        let payload = Payload::Raw(new.into());

        let node = node!(property);
        let input = input!(Some(&payload));

        let state = run!(&node, &ctx, &input);
        assert_eq!(done!(), state);
        assert_eq!(Some(new.into()), ctx.get(property));
    }

    #[test]
    fn set_property_from_error() {
        let property = "test.property";
        let err = "my error";

        let ctx = Mock::new();

        let payload = Payload::Error(err.into());

        let node = node!(property);
        let state = run!(&node, &ctx, &input!(Some(&payload)));

        assert_eq!(fail!(Some(payload)), state);
    }
}
