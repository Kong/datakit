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
