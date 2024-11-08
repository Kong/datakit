use handlebars::Handlebars;
use proxy_wasm::traits::*;
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;

use crate::config::get_config_value;
use crate::data::{Input, State};
use crate::nodes::{Node, NodeConfig, NodeFactory, PortConfig};
use crate::payload::Payload;

#[derive(Clone, Debug)]
pub struct HandlebarsConfig {
    template: String,
    content_type: String,
    inputs: Vec<String>,
}

impl NodeConfig for HandlebarsConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct HandlebarsNode<'a> {
    config: HandlebarsConfig,
    handlebars: Handlebars<'a>,
}

impl HandlebarsNode<'_> {
    fn new(config: HandlebarsConfig) -> Self {
        let mut handlebars = Handlebars::new();

        match handlebars.register_template_string("template", &config.template) {
            Ok(()) => {}
            Err(err) => {
                log::error!("handlebars: error registering template: {err}");
            }
        }

        HandlebarsNode { config, handlebars }
    }
}

impl Node for HandlebarsNode<'_> {
    fn run(&self, _ctx: &dyn HttpContext, input: &Input) -> State {
        let mut vs = Vec::new();
        let mut data = BTreeMap::new();

        for (input_name, input) in self.config.inputs.iter().zip(input.data.iter()) {
            match input {
                Some(Payload::Json(value)) => {
                    data.insert(input_name, value);
                }
                Some(Payload::Raw(vec_bytes)) => {
                    match std::str::from_utf8(vec_bytes) {
                        Ok(s) => {
                            let v = serde_json::to_value::<String>(s.into())
                                .expect("valid UTF-8 string");
                            vs.push((input_name, v));
                        }
                        Err(err) => {
                            log::error!("handlebars: input string is not valid UTF-8: {err}");
                        }
                    };
                }
                Some(Payload::Error(error)) => {
                    vs.push((input_name, serde_json::json!(error)));
                }
                None => {}
            }
        }

        for (input_name, v) in vs.iter() {
            data.insert(input_name, v);
        }

        match self.handlebars.render("template", &data) {
            Ok(output) => {
                log::debug!("output: {output}");
                match Payload::from_bytes(output.into(), Some(&self.config.content_type)) {
                    p @ Some(Payload::Error(_)) => State::Fail(vec![p]),
                    p => State::Done(vec![p]),
                }
            }
            Err(err) => State::Fail(vec![Some(Payload::Error(format!(
                "handlebars: error rendering template: {err}"
            )))]),
        }
    }
}

pub struct HandlebarsFactory {}

impl NodeFactory for HandlebarsFactory {
    fn default_input_ports(&self) -> PortConfig {
        PortConfig {
            defaults: None,
            user_defined_ports: true,
        }
    }

    fn default_output_ports(&self) -> PortConfig {
        PortConfig {
            defaults: PortConfig::names(&["output"]),
            user_defined_ports: false,
        }
    }

    fn new_config(
        &self,
        _name: &str,
        inputs: &[String],
        _outputs: &[String],
        bt: &BTreeMap<String, Value>,
    ) -> Result<Box<dyn NodeConfig>, String> {
        Ok(Box::new(HandlebarsConfig {
            inputs: inputs.to_vec(),
            template: get_config_value(bt, "template").unwrap_or_else(|| String::from("")),
            content_type: get_config_value(bt, "content_type")
                .unwrap_or_else(|| String::from("text/plain")),
        }))
    }

    fn new_node(&self, config: &dyn NodeConfig) -> Box<dyn Node> {
        match config.as_any().downcast_ref::<HandlebarsConfig>() {
            Some(cc) => Box::new(HandlebarsNode::new(cc.clone())),
            None => panic!("incompatible NodeConfig"),
        }
    }
}
