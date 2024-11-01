use crate::nodes;
use crate::nodes::{NodeConfig, NodeVec, PortConfig};
use crate::DependencyGraph;
use derivative::Derivative;
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use serde_json_wasm::de;
use std::collections::BTreeMap;
use std::fmt::{self, Formatter};

pub struct ImplicitNode {
    name: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

impl ImplicitNode {
    pub fn new(name: &str, inputs: &[&str], outputs: &[&str]) -> ImplicitNode {
        ImplicitNode {
            name: name.into(),
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            outputs: outputs.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(PartialEq, Debug)]
struct UserNodePort {
    node: Option<String>,
    port: Option<String>,
}

impl std::fmt::Display for UserNodePort {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}",
            self.node.as_deref().unwrap_or(""),
            self.port.as_deref().unwrap_or("")
        )
    }
}

#[derive(PartialEq, Debug)]
struct UserLink {
    from: UserNodePort,
    to: UserNodePort,
}

#[derive(PartialEq, Debug)]
struct UserNodeDesc {
    node_type: String,
    name: String,
}

#[derive(PartialEq, Debug)]
struct UserNodeConfig {
    desc: UserNodeDesc,
    bt: BTreeMap<String, serde_json::Value>,
    links: Vec<UserLink>,
    n_inputs: usize,
    n_outputs: usize,
    named_ins: Vec<String>,
    named_outs: Vec<String>,
}

impl UserLink {
    pub fn new(
        from_node: Option<String>,
        from_port: Option<String>,
        to_node: Option<String>,
        to_port: Option<String>,
    ) -> Self {
        UserLink {
            from: UserNodePort {
                node: from_node,
                port: from_port,
            },
            to: UserNodePort {
                node: to_node,
                port: to_port,
            },
        }
    }

    pub fn new_reverse(
        from_node: Option<String>,
        from_port: Option<String>,
        to_node: Option<String>,
        to_port: Option<String>,
    ) -> Self {
        UserLink {
            from: UserNodePort {
                node: to_node,
                port: to_port,
            },
            to: UserNodePort {
                node: from_node,
                port: from_port,
            },
        }
    }
}

fn parse_node_port(value: String) -> (Option<String>, Option<String>) {
    let trim = value.trim().to_string();

    if let Some(dot) = trim.find('.') {
        let (node, port) = trim.split_at(dot);
        (
            Some(node.trim().to_string()),
            Some(port[1..].trim().to_string()),
        )
    } else {
        (Some(trim), None)
    }
}

impl<'a> Deserialize<'a> for UserNodeConfig {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct UserNodeConfigVisitor;

        impl<'de> Visitor<'de> for UserNodeConfigVisitor {
            type Value = UserNodeConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a DataKit node config")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut bt = BTreeMap::new();
                let mut typ: Option<String> = None;
                let mut name: Option<String> = None;
                let mut links: Vec<UserLink> = Vec::new();
                let mut named_ins: Vec<String> = Vec::new();
                let mut named_outs: Vec<String> = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            if let Ok(serde_json::Value::String(value)) = map.next_value() {
                                typ = Some(value);
                            }
                        }
                        "name" => {
                            if let Ok(serde_json::Value::String(value)) = map.next_value() {
                                name = Some(value);
                            }
                        }
                        "input" => {
                            if let Ok(serde_json::Value::String(node_port)) = map.next_value() {
                                let (node, port) = parse_node_port(node_port);
                                links.push(UserLink::new(node, port, None, None));
                            }
                        }
                        "inputs" => {
                            if let Ok(v) = map.next_value::<serde_json::Value>() {
                                read_links(&mut links, v, &mut named_ins, &UserLink::new)
                                    .map_err(Error::custom::<&str>)?;
                            }
                        }
                        "output" => {
                            if let Ok(serde_json::Value::String(value)) = map.next_value() {
                                let (node, port) = parse_node_port(value);
                                links.push(UserLink::new(None, None, node, port));
                            }
                        }
                        "outputs" => {
                            if let Ok(v) = map.next_value::<serde_json::Value>() {
                                read_links(&mut links, v, &mut named_outs, &UserLink::new_reverse)
                                    .map_err(Error::custom::<&str>)?;
                            }
                        }
                        _ => {
                            if let Ok(value) = map.next_value() {
                                bt.insert(key, value);
                            }
                        }
                    }
                }

                let name = name.unwrap_or_else(|| format!("{:p}", &bt));

                let mut n_inputs = 0;
                let mut n_outputs = 0;
                for link in &mut links {
                    if link.to.node.is_none() {
                        link.to.node = Some(name.clone());
                        n_inputs += 1;
                    }
                    if link.from.node.is_none() {
                        link.from.node = Some(name.clone());
                        n_outputs += 1;
                    }
                }

                if let Some(node_type) = typ {
                    Ok(UserNodeConfig {
                        desc: UserNodeDesc { node_type, name },
                        bt,
                        links,
                        n_inputs,
                        n_outputs,
                        named_ins,
                        named_outs,
                    })
                } else {
                    Err(Error::missing_field("type"))
                }
            }
        }

        de.deserialize_map(UserNodeConfigVisitor)
    }
}

fn read_links(
    links: &mut Vec<UserLink>,
    value: Value,
    named: &mut Vec<String>,
    ctor: &impl Fn(Option<String>, Option<String>, Option<String>, Option<String>) -> UserLink,
) -> Result<(), &'static str> {
    if value.is_object() {
        if let Ok(map) = serde_json::from_value::<Map<String, serde_json::Value>>(value) {
            for (my_port, v) in map {
                named.push(my_port.clone());
                if let Ok(node_port) = serde_json::from_value::<String>(v) {
                    let (node, port) = parse_node_port(node_port);
                    links.push(ctor(node, port, None, Some(my_port)));
                } else {
                    return Err("invalid map value");
                }
            }
        } else {
            return Err("invalid map");
        }
    } else if value.is_array() {
        if let Ok(vec) = serde_json::from_value::<Vec<serde_json::Value>>(value) {
            for v in vec {
                if v.is_object() {
                    read_links(links, v, named, ctor)?;
                } else if let Ok(node_port) = serde_json::from_value::<String>(v) {
                    let (node, port) = parse_node_port(node_port);
                    links.push(ctor(node, port, None, None));
                } else {
                    return Err("invalid list value");
                }
            }
        } else {
            return Err("invalid list");
        }
    } else {
        return Err("invalid object");
    }
    Ok(())
}

#[derive(Deserialize, Default, PartialEq, Debug)]
pub struct UserConfig {
    nodes: Vec<UserNodeConfig>,
    #[serde(default)]
    debug: bool,
}

#[derive(Derivative)]
#[derivative(PartialEq, Debug)]
struct NodeInfo {
    name: String,
    node_type: String,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Debug = "ignore")]
    node_config: Box<dyn NodeConfig>,
}

#[derive(PartialEq, Debug)]
pub struct Config {
    n_nodes: usize,
    n_implicits: usize,
    node_list: Vec<NodeInfo>,
    graph: DependencyGraph,
    debug: bool,
}

fn add_default_links(
    name: &str,
    n_inputs: usize,
    n_outputs: usize,
    links: &mut Vec<UserLink>,
    nc: &dyn NodeConfig,
) {
    if n_inputs == 0 {
        if let Some(default_inputs) = nc.default_inputs() {
            for input in &default_inputs {
                links.push(UserLink {
                    from: UserNodePort {
                        node: Some(input.other_node.clone()),
                        port: Some(input.other_port.clone()),
                    },
                    to: UserNodePort {
                        node: Some(name.into()),
                        port: Some(input.this_port.clone()),
                    },
                });
            }
        }
    }
    if n_outputs == 0 {
        if let Some(default_outputs) = nc.default_outputs() {
            for output in &default_outputs {
                links.push(UserLink {
                    from: UserNodePort {
                        node: Some(name.into()),
                        port: Some(output.this_port.clone()),
                    },
                    to: UserNodePort {
                        node: Some(output.other_node.clone()),
                        port: Some(output.other_port.clone()),
                    },
                });
            }
        }
    }
}

fn push_ports(ports: &mut Vec<Vec<String>>, pc: PortConfig, given: &Vec<String>) -> bool {
    let mut list = pc.defaults.unwrap_or_default();

    if pc.user_defined_ports {
        for port in given {
            if !list.iter().any(|p| p == port) {
                list.push(port.into());
            }
        }
    }

    ports.push(list);

    pc.user_defined_ports
}

fn has(xs: &[String], s: &str) -> bool {
    xs.iter().any(|x| x == s)
}

fn resolve_port_names(
    link: &mut UserLink,
    outs: &mut Vec<String>,
    user_outs: bool,
    ins: &mut Vec<String>,
    user_ins: bool,
    n_linked_inputs: usize,
) -> Result<(), String> {
    let mut from_port = None;
    let mut to_port = None;
    match &link.from.port {
        Some(port) => {
            if !has(outs, port) {
                if user_outs {
                    outs.push(port.into());
                } else {
                    return Err(format!("invalid output port name {port}"));
                }
            }
        }
        None => {
            // If out ports list has a first port declared (either explicitly
            // or implicitly), use it
            if let Some(&port) = outs.first().as_ref() {
                from_port = Some(port.into());
            } else if user_outs {
                let new_port = make_port_name(&link.to)?;

                // otherwise the if outs.first() would have returned it
                assert!(!has(outs, &new_port));

                from_port = Some(new_port.clone());
                outs.push(new_port);
            } else {
                return Err("node in link has no output ports".into());
            }
        }
    }

    match &link.to.port {
        Some(port) => {
            if !has(ins, port) {
                if user_ins {
                    ins.push(port.into());
                } else {
                    return Err(format!("invalid input port name {port}"));
                }
            }
        }
        None => {
            if user_ins {
                let new_port = make_port_name(&link.from)?;
                if !has(outs, &new_port) {
                    to_port = Some(new_port.clone());
                    ins.push(new_port);
                } else {
                    return Err(format!("duplicated input port {new_port}"));
                }
            } else if let Some(&port) = ins.get(n_linked_inputs - 1).as_ref() {
                to_port = Some(port.into());
            } else {
                let n = ins.len();
                return Err(format!(
                    "too many inputs declared (node type supports {n} inputs)"
                ));
            }
        }
    }

    // assign in the end, so that the input and output resolution
    // are not affected by the order of links when calling make_port_name
    if from_port.is_some() {
        link.from.port = from_port;
    }
    if to_port.is_some() {
        link.to.port = to_port;
    }
    assert!(link.from.port.is_some());
    assert!(link.to.port.is_some());

    Ok(())
}

fn make_port_name(np: &UserNodePort) -> Result<String, String> {
    Ok(match (&np.node, &np.port) {
        (Some(n), Some(p)) => format!("{n}.{p}"),
        (Some(n), None) => n.into(),
        (None, _) => return Err("could not resolve a name".into()),
    })
}

fn err_at_node(desc: &UserNodeDesc, e: &str) -> String {
    let name = &desc.name;
    let nt = &desc.node_type;
    format!("in node `{name}` of type `{nt}`: {e}")
}

fn get_link_str(o: &Option<String>, _name: &str) -> Result<String, String> {
    o.as_ref()
        .ok_or_else(|| "bad link definition in node {_name}".into())
        .cloned()
}

fn convert_config(
    mut user_config: UserConfig,
    implicits: &[ImplicitNode],
) -> Result<Config, String> {
    let p = implicits.len();
    let n = user_config.nodes.len() + p;

    let mut node_names: Vec<String> = Vec::with_capacity(n);
    let mut in_ports = Vec::with_capacity(n);
    let mut out_ports = Vec::with_capacity(n);
    let mut user_def_ins = vec![true; n];
    let mut user_def_outs = vec![true; n];
    let mut node_list = Vec::with_capacity(n);

    // This is performed in several loops to ensure that the resolution
    // order for links does not depend on the order of the nodes given
    // in the input file.

    for (i, inode) in implicits.iter().enumerate() {
        node_names.push(inode.name.clone());
        in_ports.push(inode.inputs.clone());
        out_ports.push(inode.outputs.clone());
        user_def_ins[i] = false;
        user_def_outs[i] = false;
        node_list.push(NodeInfo {
            name: inode.name.clone(),
            node_type: "implicit".into(),
            node_config: Box::new(nodes::implicit::ImplicitConfig {}),
        });
    }

    for (i, unc) in user_config.nodes.iter().enumerate() {
        let name: &str = &unc.desc.name;
        let nt: &str = &unc.desc.node_type;

        // at this point, node_names contains only the implicit entries
        if has(&node_names, name) {
            return Err(format!("cannot use reserved node name `{name}`"));
        }

        if !nodes::is_valid_type(nt) {
            return Err(format!("unknown node type `{nt}`"));
        }

        let ins = nodes::default_input_ports(nt).unwrap();
        user_def_ins[i + p] = push_ports(&mut in_ports, ins, &unc.named_ins);

        let outs = nodes::default_output_ports(nt).unwrap();
        user_def_outs[i + p] = push_ports(&mut out_ports, outs, &unc.named_outs);
    }

    for unc in &user_config.nodes {
        let name = &unc.desc.name;

        if node_names.iter().any(|n| n == name) {
            return Err(format!("multiple definitions of node `{name}`"));
        }

        node_names.push(name.into());
    }

    let mut linked_inputs = vec![0; n];
    for unc in user_config.nodes.iter_mut() {
        for link in &mut unc.links {
            let s = node_position(&node_names, &link.from, &unc.desc)?;
            let d = node_position(&node_names, &link.to, &unc.desc)?;
            let outs = &mut out_ports[s];
            let u_outs = user_def_outs[s];
            let ins = &mut in_ports[d];
            let u_ins = user_def_ins[d];

            linked_inputs[d] += 1;

            resolve_port_names(link, outs, u_outs, ins, u_ins, linked_inputs[d])
                .map_err(|e| err_at_node(&unc.desc, &e))?;
        }
    }

    for (u, unc) in user_config.nodes.iter_mut().enumerate() {
        let i = u + p;
        let ins = &mut in_ports[i];
        let outs = &mut out_ports[i];
        let name = &unc.desc.name;
        let desc = &unc.desc;
        match nodes::new_config(&desc.node_type, &desc.name, ins, outs, &unc.bt) {
            Ok(nc) => {
                add_default_links(name, unc.n_inputs, unc.n_outputs, &mut unc.links, &*nc);

                node_list.push(NodeInfo {
                    name: name.to_string(),
                    node_type: desc.node_type.to_string(),
                    node_config: nc,
                });
            }
            Err(err) => return Err(err),
        };
    }

    let mut graph = DependencyGraph::new(node_names, in_ports, out_ports);

    for unc in &user_config.nodes {
        let name = &unc.desc.name;
        for link in &unc.links {
            graph.add(
                &get_link_str(&link.from.node, name)?,
                &get_link_str(&link.from.port, name)?,
                &get_link_str(&link.to.node, name)?,
                &get_link_str(&link.to.port, name)?,
            )?;
        }
    }

    Ok(Config {
        n_nodes: n,
        n_implicits: p,
        node_list,
        graph,
        debug: user_config.debug,
    })
}

fn node_position(
    node_names: &[String],
    np: &UserNodePort,
    desc: &UserNodeDesc,
) -> Result<usize, String> {
    node_names
        .iter()
        .position(|name: &String| Some(name) == np.node.as_ref())
        .ok_or_else(|| err_at_node(desc, &format!("unknown node in link: {}", np)))
}

impl Config {
    pub fn new(config_bytes: Vec<u8>, implicits: &[ImplicitNode]) -> Result<Config, String> {
        match de::from_slice::<UserConfig>(&config_bytes) {
            Ok(user_config) => convert_config(user_config, implicits)
                .map_err(|err| format!("failed checking configuration: {err}")),
            Err(err) => Err(format!("failed parsing configuration: {err}",)),
        }
    }

    pub fn debug(&self) -> bool {
        self.debug
    }

    pub fn node_count(&self) -> usize {
        self.n_nodes
    }

    pub fn number_of_implicits(&self) -> usize {
        self.n_implicits
    }

    pub fn node_types(&self) -> impl Iterator<Item = (&str, &str)> {
        self.node_list
            .iter()
            .map(|info| (info.name.as_ref(), info.node_type.as_ref()))
    }

    pub fn get_graph(&self) -> &DependencyGraph {
        &self.graph
    }

    pub fn build_nodes(&self) -> NodeVec {
        let mut nodes = NodeVec::with_capacity(self.node_list.len());

        for info in &self.node_list {
            match nodes::new_node(&info.node_type, &*info.node_config) {
                Ok(node) => nodes.push(node),
                Err(err) => log::error!("{err}"),
            }
        }

        nodes
    }
}

pub fn get_config_value<T: for<'de> serde::Deserialize<'de>>(
    bt: &BTreeMap<String, Value>,
    key: &str,
) -> Option<T> {
    bt.get(key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;
    use std::any::Any;

    fn deserialize_user_config(cfg: &str) -> UserConfig {
        de::from_slice::<UserConfig>(cfg.as_bytes()).unwrap()
    }

    #[test]
    fn deserialize_empty_nodes() {
        let uc = deserialize_user_config(
            r#"{
                "nodes": []
            }"#,
        );
        assert_eq!(
            uc,
            UserConfig {
                nodes: vec![],
                debug: false,
            }
        );
    }

    #[test]
    fn deserialize_complete_example() {
        let uc = deserialize_user_config(
            r#"{
                "nodes": [
                    {
                        "name": "jq1",
                        "type": "jq",
                        "input": "request.headers",
                        "jq": "{ \"x-bar\": $request_headers[\"x-foo\"] }"
                    },
                    {
                        "name": "mycall",
                        "type": "call",
                        "input": "jq1",
                        "url": "http://example.com"
                    },
                    {
                        "name": "jq2",
                        "type": "jq",
                        "inputs": {
                            "$mycall": "mycall",
                            "$request": "request.body"
                        },
                        "jq": "{ \"bee\": $mycall.bee, \"boo\": $request.boo }"
                    }
                ]
            }"#,
        );
        assert_eq!(
            uc,
            UserConfig {
                nodes: vec![
                    UserNodeConfig {
                        desc: UserNodeDesc {
                            node_type: "jq".into(),
                            name: "jq1".into(),
                        },
                        bt: BTreeMap::from([(
                            "jq".into(),
                            json!("{ \"x-bar\": $request_headers[\"x-foo\"] }")
                        )]),
                        links: vec![UserLink {
                            from: UserNodePort {
                                node: Some("request".into()),
                                port: Some("headers".into())
                            },
                            to: UserNodePort {
                                node: Some("jq1".into()),
                                port: None
                            }
                        }],
                        n_inputs: 1,
                        n_outputs: 0,
                        named_ins: vec![],
                        named_outs: vec![]
                    },
                    UserNodeConfig {
                        desc: UserNodeDesc {
                            node_type: "call".into(),
                            name: "mycall".into()
                        },
                        bt: BTreeMap::from([("url".to_string(), json!("http://example.com"))]),
                        links: vec![UserLink {
                            from: UserNodePort {
                                node: Some("jq1".into()),
                                port: None
                            },
                            to: UserNodePort {
                                node: Some("mycall".into()),
                                port: None
                            }
                        }],
                        n_inputs: 1,
                        n_outputs: 0,
                        named_ins: vec![],
                        named_outs: vec![]
                    },
                    UserNodeConfig {
                        desc: UserNodeDesc {
                            node_type: "jq".into(),
                            name: "jq2".into()
                        },
                        bt: BTreeMap::from([(
                            "jq".to_string(),
                            json!("{ \"bee\": $mycall.bee, \"boo\": $request.boo }")
                        )]),
                        links: vec![
                            UserLink {
                                from: UserNodePort {
                                    node: Some("mycall".into()),
                                    port: None
                                },
                                to: UserNodePort {
                                    node: Some("jq2".into()),
                                    port: Some("$mycall".into())
                                }
                            },
                            UserLink {
                                from: UserNodePort {
                                    node: Some("request".into()),
                                    port: Some("body".into())
                                },
                                to: UserNodePort {
                                    node: Some("jq2".into()),
                                    port: Some("$request".into())
                                }
                            }
                        ],
                        n_inputs: 2,
                        n_outputs: 0,
                        named_ins: vec!["$mycall".into(), "$request".into()],
                        named_outs: vec![]
                    }
                ],
                debug: false
            }
        );
    }

    #[test]
    fn test_parse_node_port() {
        let cases = vec![
            ("", (Some(""), None)),
            (" ", (Some(""), None)),
            (".", (Some(""), Some(""))),
            (". ", (Some(""), Some(""))),
            (" . ", (Some(""), Some(""))),
            (".foo", (Some(""), Some("foo"))),
            (".foo.bar", (Some(""), Some("foo.bar"))),
            ("..foo.bar", (Some(""), Some(".foo.bar"))),
            (". .foo.bar", (Some(""), Some(".foo.bar"))),
            ("f.bar", (Some("f"), Some("bar"))),
            ("foo", (Some("foo"), None)),
            ("foo.", (Some("foo"), Some(""))),
            ("foo.b", (Some("foo"), Some("b"))),
            ("foo.b  ", (Some("foo"), Some("b"))),
            ("foo.bar", (Some("foo"), Some("bar"))),
            ("foo  .  bar", (Some("foo"), Some("bar"))),
            ("foo..baz", (Some("foo"), Some(".baz"))),
            ("foo.bar.", (Some("foo"), Some("bar."))),
            ("foo.bar..", (Some("foo"), Some("bar.."))),
            ("foo.bar.baz", (Some("foo"), Some("bar.baz"))),
            ("foo.bar baz", (Some("foo"), Some("bar baz"))),
            ("foo bar.baz bla", (Some("foo bar"), Some("baz bla"))),
            ("  foo . bar.baz ", (Some("foo"), Some("bar.baz"))),
        ];
        for (node_port, pair) in cases {
            assert_eq!(
                parse_node_port(node_port.to_owned()),
                (pair.0.map(str::to_owned), pair.1.map(str::to_owned))
            );
        }
    }

    fn accept_config(cfg: &str) -> Config {
        let result = Config::new(cfg.as_bytes().to_vec(), &[]);

        result.unwrap()
    }

    fn reject_config_with(cfg: &str, message: &str) {
        let result = Config::new(cfg.as_bytes().to_vec(), &[]);

        let err = result.unwrap_err();
        assert_eq!(err, message);
    }

    #[test]
    fn config_no_json() {
        reject_config_with(
            "",
            "failed parsing configuration: EOF while parsing a JSON value.",
        )
    }

    #[test]
    fn config_bad_json() {
        reject_config_with(
            "{",
            "failed parsing configuration: EOF while parsing an object.",
        )
    }

    #[test]
    fn config_empty_json() {
        reject_config_with("{}", "failed parsing configuration: missing field `nodes`")
    }

    #[test]
    fn config_empty_nodes() {
        accept_config(
            r#"{
                "nodes": []
            }"#,
        );
    }

    #[test]
    fn config_missing_type() {
        reject_config_with(
            r#"{
                "nodes": [
                    {
                        "name": "MY_NODE"
                    }
                ]
            }"#,
            "failed parsing configuration: missing field `type`",
        )
    }

    #[test]
    fn config_invalid_type() {
        reject_config_with(
            r#"{
                "nodes": [
                    {
                        "name": "MY_NODE",
                        "type": "INVALID"
                    }
                ]
            }"#,
            "failed checking configuration: unknown node type `INVALID`",
        )
    }

    struct IgnoreConfig {}
    impl NodeConfig for IgnoreConfig {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn convert_complete_example() {
        let uc = deserialize_user_config(
            r#"{
                "nodes": [
                    {
                        "name": "jq1",
                        "type": "jq",
                        "input": "request.headers",
                        "jq": "{ \"x-bar\": $request_headers[\"x-foo\"] }"
                    },
                    {
                        "name": "mycall",
                        "type": "call",
                        "input": "jq1",
                        "url": "http://example.com"
                    },
                    {
                        "name": "jq2",
                        "type": "jq",
                        "inputs": {
                            "$mycall": "mycall",
                            "$request": "request.body"
                        },
                        "jq": "{ \"bee\": $mycall.bee, \"boo\": $request.boo }"
                    }
                ]
            }"#,
        );

        nodes::register_node("call", Box::new(nodes::call::CallFactory {}));
        nodes::register_node("jq", Box::new(nodes::jq::JqFactory {}));

        let implicits = vec![
            ImplicitNode::new("request", &[], &["body", "headers"]),
            ImplicitNode::new("service_request", &["body", "headers"], &[]),
            ImplicitNode::new("response", &["body", "headers"], &[]),
            ImplicitNode::new("service_response", &[], &["body", "headers"]),
        ];

        let config = convert_config(uc, &implicits).unwrap();
        assert!(!config.debug);
        assert_eq!(config.n_nodes, 7);
        assert_eq!(config.n_implicits, 4);
        assert_eq!(
            config.node_list,
            vec![
                NodeInfo {
                    name: "request".into(),
                    node_type: "implicit".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "service_request".into(),
                    node_type: "implicit".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "response".into(),
                    node_type: "implicit".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "service_response".into(),
                    node_type: "implicit".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "jq1".into(),
                    node_type: "jq".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "mycall".into(),
                    node_type: "call".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "jq2".into(),
                    node_type: "jq".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
            ]
        );
        let input_lists: &[&[Option<(usize, usize)>]] = &[
            &[],
            &[None, None],
            &[None, None],
            &[],
            &[Some((0, 1))],
            &[Some((4, 0)), None, None],
            &[Some((5, 0)), Some((0, 0))],
        ];
        for (i, &input_list) in input_lists.iter().enumerate() {
            let given: Vec<_> = input_list.iter().collect();
            let computed: Vec<_> = config.graph.each_input(i).collect();
            assert_eq!(given, computed);
        }

        let output_lists: &[&[&[(usize, usize)]]] = &[
            &[&[(6, 1)], &[(4, 0)]],
            &[],
            &[],
            &[&[], &[]],
            &[&[(5, 0)]],
            &[&[(6, 0)], &[]],
            &[],
        ];
        for (i, &output_list) in output_lists.iter().enumerate() {
            let given: Vec<_> = output_list.iter().collect();
            let computed: Vec<_> = config.graph.each_output(i).collect();
            assert_eq!(given, computed);
        }
    }
}
