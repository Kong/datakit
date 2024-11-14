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
}

impl PropertyConfig {
    fn new(name: String) -> Self {
        Self {
            path: name.split('.').map(|s| s.to_string()).collect(),
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
    fn run(&self, ctx: &dyn HttpContext, _input: &Input) -> State {
        log::debug!("property: run");

        Done(match ctx.get_property(self.config.to_path()) {
            Some(bytes) => {
                log::info!("{:?} => {:?}", &self.config.path, bytes);
                vec![Some(Payload::Raw(bytes))]
            }
            None => {
                log::info!("{:?} => None", &self.config.path);
                vec![]
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
        let name = get_config_value::<String>(bt, "property")
            .ok_or_else(|| "Missing `property` attribute".to_owned())?;

        Ok(Box::new(PropertyConfig::new(name)))
    }

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<PropertyConfig>() {
            Some(cc) => Box::new(Property { config: cc.clone() }),
            None => panic!("incompatible NodeConfig"),
        }
    }
}
