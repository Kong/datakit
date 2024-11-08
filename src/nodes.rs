use proxy_wasm::traits::*;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use crate::data::{Input, State, State::*};

pub mod call;
pub mod exit;
pub mod jq;
pub mod template;

pub type NodeVec = Vec<Box<dyn Node>>;

#[derive(Clone, Debug)]
pub struct PortConfig {
    pub defaults: Option<Vec<String>>,
    pub user_defined_ports: bool,
}

impl PortConfig {
    fn names(list: &[&str]) -> Option<Vec<String>> {
        Some(list.iter().map(|&s| str::to_owned(s)).collect())
    }

    /// Combine defaults and user-given ports
    /// into the final ordered list of ports.
    pub fn into_port_list(self: PortConfig, given: &[String]) -> Vec<String> {
        let mut list = self.defaults.unwrap_or_default();

        if self.user_defined_ports {
            for port in given {
                if !list.iter().any(|p| p == port) {
                    list.push(port.into());
                }
            }
        }

        list
    }
}

pub trait Node {
    fn run(&self, _ctx: &dyn HttpContext, _input: &Input) -> State {
        Done(vec![None])
    }

    fn resume(&self, _ctx: &dyn HttpContext, _input: &Input) -> State {
        Done(vec![None])
    }
}

pub struct NodeDefaultLink {
    pub this_port: String,
    pub other_node: String,
    pub other_port: String,
}

pub trait NodeConfig {
    fn as_any(&self) -> &dyn Any;

    fn default_inputs(&self) -> Option<Vec<NodeDefaultLink>> {
        None
    }

    fn default_outputs(&self) -> Option<Vec<NodeDefaultLink>> {
        None
    }
}

pub trait NodeFactory: Send {
    fn new_config(
        &self,
        name: &str,
        inputs: &[String],
        outputs: &[String],
        bt: &BTreeMap<String, Value>,
    ) -> Result<Box<dyn NodeConfig>, String>;

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node>;

    fn default_input_ports(&self) -> PortConfig;

    fn default_output_ports(&self) -> PortConfig;
}

type NodeTypeMap = BTreeMap<String, Box<dyn NodeFactory>>;

fn node_types() -> &'static Mutex<NodeTypeMap> {
    static NODE_TYPES: OnceLock<Mutex<NodeTypeMap>> = OnceLock::new();
    NODE_TYPES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub fn register_node(name: &str, factory: Box<dyn NodeFactory>) {
    node_types().lock().unwrap().insert(name.into(), factory);
}

fn with_node_type<T>(node_type: &str, f: impl Fn(&Box<dyn NodeFactory>) -> T) -> Option<T>
where
    T: Sized,
{
    node_types().lock().unwrap().get(node_type).map(f)
}

pub fn is_valid_type(node_type: &str) -> bool {
    with_node_type(node_type, |_| true).unwrap_or(false)
}

pub fn default_input_ports(node_type: &str) -> Option<PortConfig> {
    with_node_type(node_type, |nf| nf.default_input_ports())
}

pub fn default_output_ports(node_type: &str) -> Option<PortConfig> {
    with_node_type(node_type, |nf| nf.default_output_ports())
}

pub fn new_config(
    node_type: &str,
    name: &str,
    inputs: &[String],
    outputs: &[String],
    bt: &BTreeMap<String, Value>,
) -> Result<Box<dyn NodeConfig>, String> {
    match with_node_type(node_type, |nf| nf.new_config(name, inputs, outputs, bt)) {
        Some(Ok(ok)) => Ok(ok),
        Some(Err(e)) => Err(e),
        None => Err(format!("no such node type: {node_type}")),
    }
}

pub fn new_node(node_type: &str, config: &dyn NodeConfig) -> Result<Box<dyn Node>, String> {
    with_node_type(node_type, |nf| nf.new_node(config))
        .ok_or(format!("no such node type: {node_type}"))
}

pub mod implicit {
    use super::*;

    #[derive(Clone)]
    pub struct Implicit {}

    impl Node for Implicit {}

    pub struct SourceFactory {}
    pub struct SinkFactory {}

    #[derive(Debug)]
    pub struct ImplicitConfig {}

    impl NodeConfig for ImplicitConfig {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl NodeFactory for SourceFactory {
        fn default_input_ports(&self) -> PortConfig {
            PortConfig {
                defaults: None,
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
            _bt: &BTreeMap<String, Value>,
        ) -> Result<Box<dyn NodeConfig>, String> {
            Ok(Box::new(ImplicitConfig {}))
        }

        fn new_node(&self, _config: &dyn NodeConfig) -> Box<dyn Node> {
            Box::new(Implicit {})
        }
    }

    impl NodeFactory for SinkFactory {
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
            _bt: &BTreeMap<String, Value>,
        ) -> Result<Box<dyn NodeConfig>, String> {
            Ok(Box::new(ImplicitConfig {}))
        }

        fn new_node(&self, _config: &dyn NodeConfig) -> Box<dyn Node> {
            Box::new(Implicit {})
        }
    }
}
