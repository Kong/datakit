use crate::nodes;
use crate::nodes::{NodeConfig, NodeVec};
use crate::DependencyGraph;
use derivative::Derivative;
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use serde_json_wasm::de;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{self, Formatter};

pub struct ImplicitNode {
    name: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

impl ImplicitNode {
    pub fn new(name: &str, inputs: Vec<String>, outputs: Vec<String>) -> ImplicitNode {
        ImplicitNode {
            name: name.into(),
            inputs,
            outputs,
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

    fn accept_port_name(port: &String, ports: &mut Vec<String>, user: bool) -> bool {
        if ports.contains(port) {
            true
        } else if user {
            ports.push(port.into());
            true
        } else {
            false
        }
    }

    fn get_or_create_output(
        np: &UserNodePort,
        outs: &mut Vec<String>,
        user: bool,
    ) -> Result<String, String> {
        // If out ports list has a first port declared
        // (either explicitly or implicitly), use it
        if let Some(&port) = outs.first().as_ref() {
            Ok(port.into())
        } else if user {
            let new_port = make_port_name(np)?;

            // otherwise the if outs.first() would have returned it
            assert!(!outs.contains(&new_port));

            outs.push(new_port.clone());
            Ok(new_port)
        } else {
            Err("node in link has no output ports".into())
        }
    }

    fn create_or_get_input(
        np: &UserNodePort,
        ins: &mut Vec<String>,
        user: bool,
        n: usize,
    ) -> Result<String, String> {
        if user {
            let new_port = make_port_name(np)?;
            if ins.contains(&new_port) {
                return Err(format!("duplicated input port {new_port}"));
            }
            ins.push(new_port.clone());
            Ok(new_port.clone())
        } else if let Some(&port) = ins.get(n - 1).as_ref() {
            Ok(port.into())
        } else {
            Err(format!(
                "too many inputs declared (node type supports {} inputs)",
                ins.len()
            ))
        }
    }

    fn resolve_port_names(
        self: &mut UserLink,
        src: &mut PortInfo,
        dst: &mut PortInfo,
        n_ins: usize,
    ) -> Result<(), String> {
        let mut from_port = None;
        let mut to_port = None;

        let outs = &mut src.outs;
        let user_outs = src.user_outs;
        let ins = &mut dst.ins;
        let user_ins = dst.user_ins;

        match &self.from.port {
            Some(port) => {
                if !Self::accept_port_name(port, outs, user_outs) {
                    let node = self.from.node.as_ref().unwrap();
                    return Err(format!("invalid output port name {node}.{port}"));
                }
            }
            None => {
                from_port = Some(Self::get_or_create_output(&self.to, outs, user_outs)?);
            }
        }

        match &self.to.port {
            Some(port) => {
                if !Self::accept_port_name(port, ins, user_ins) {
                    let node = self.to.node.as_ref().unwrap();
                    return Err(format!("invalid input port name {node}.{port}"));
                }
            }
            None => {
                to_port = Some(Self::create_or_get_input(&self.from, ins, user_ins, n_ins)?);
            }
        }

        // assign in the end, so that the input and output resolution
        // are not affected by the order of links when calling make_port_name
        if from_port.is_some() {
            self.from.port = from_port;
        }
        if to_port.is_some() {
            self.to.port = to_port;
        }
        assert!(self.from.port.is_some());
        assert!(self.to.port.is_some());

        Ok(())
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
    match value {
        Value::Object(map) => {
            for (my_port, v) in map {
                named.push(my_port.clone());

                let Value::String(node_port) = v else {
                    return Err("invalid map value");
                };

                let (node, port) = parse_node_port(node_port);
                links.push(ctor(node, port, None, Some(my_port)));
            }
        }

        Value::Array(vec) => {
            for v in vec {
                match v {
                    Value::Object(map) => {
                        read_links(links, map.into(), named, ctor)?;
                    }

                    Value::String(node_port) => {
                        let (node, port) = parse_node_port(node_port);
                        links.push(ctor(node, port, None, None));
                    }

                    _ => {
                        return Err("invalid list value");
                    }
                }
            }
        }

        _ => return Err("invalid object"),
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

struct PortInfo {
    ins: Vec<String>,
    outs: Vec<String>,
    user_ins: bool,
    user_outs: bool,
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

impl PortInfo {
    fn new(node_type: &str, named_ins: &[String], named_outs: &[String]) -> Self {
        let ins_pc = nodes::default_input_ports(node_type).unwrap();
        let outs_pc = nodes::default_output_ports(node_type).unwrap();
        PortInfo {
            user_ins: ins_pc.user_defined_ports,
            user_outs: outs_pc.user_defined_ports,
            ins: ins_pc.into_port_list(named_ins),
            outs: outs_pc.into_port_list(named_outs),
        }
    }
}

fn node_position(node_names: &[String], np: &UserNodePort) -> Result<usize, String> {
    node_names
        .iter()
        .position(|name: &String| Some(name) == np.node.as_ref())
        .ok_or_else(|| format!("unknown node in link: {}", np))
}

fn get_source_dest_ports(
    port_list: &mut [PortInfo],
    s: usize,
    d: usize,
) -> Result<(&mut PortInfo, &mut PortInfo), &'static str> {
    match s.cmp(&d) {
        Ordering::Less => {
            let (ss, ds) = port_list.split_at_mut(s + 1);
            Ok((&mut ss[s], &mut ds[d - (s + 1)]))
        }
        Ordering::Greater => {
            let (ds, ss) = port_list.split_at_mut(d + 1);
            Ok((&mut ss[s - (d + 1)], &mut ds[d]))
        }
        Ordering::Equal => Err("node cannot connect to itself"),
    }
}

fn fixup_missing_port_names(
    unc: &mut UserNodeConfig,
    node_names: &[String],
    port_list: &mut [PortInfo],
    linked_inputs: &mut [usize],
) -> Result<(), String> {
    for link in &mut unc.links {
        let s = node_position(node_names, &link.from)?;
        let d = node_position(node_names, &link.to)?;
        let (src, dst) = get_source_dest_ports(port_list, s, d)?;

        linked_inputs[d] += 1;

        link.resolve_port_names(src, dst, linked_inputs[d])?;
    }
    Ok(())
}

fn make_node_info(unc: &mut UserNodeConfig, port_info: &PortInfo) -> Result<NodeInfo, String> {
    let name = &unc.desc.name;
    let node_type = &unc.desc.node_type;

    let nc = nodes::new_config(node_type, name, &port_info.ins, &port_info.outs, &unc.bt)?;

    add_default_links(name, unc.n_inputs, unc.n_outputs, &mut unc.links, &*nc);

    Ok(NodeInfo {
        name: name.to_string(),
        node_type: node_type.to_string(),
        node_config: nc,
    })
}

fn into_name_lists(ports: Vec<PortInfo>) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let n = ports.len();
    let mut input_names = Vec::with_capacity(n);
    let mut output_names = Vec::with_capacity(n);
    for pi in ports.into_iter() {
        input_names.push(pi.ins);
        output_names.push(pi.outs);
    }
    (input_names, output_names)
}

impl UserConfig {
    fn into_config(mut self, implicits: &[ImplicitNode]) -> Result<Config, String> {
        let p = implicits.len();
        let n = self.nodes.len() + p;

        let mut node_names: Vec<String> = Vec::with_capacity(n);
        let mut nodes = Vec::with_capacity(n);
        let mut ports = Vec::with_capacity(n);

        // This is performed in several loops to ensure that the resolution
        // order for links does not depend on the order of the nodes given
        // in the input file.

        for inode in implicits.iter() {
            node_names.push(inode.name.clone());
            nodes.push(NodeInfo {
                name: inode.name.clone(),
                node_type: "implicit".into(),
                node_config: Box::new(nodes::implicit::ImplicitConfig {}),
            });
            ports.push(PortInfo::new("implicit", &inode.inputs, &inode.outputs));
        }

        for unc in &self.nodes {
            let desc = &unc.desc;
            let name = &desc.name;
            let node_type = &desc.node_type;

            // at this point, node_names contains only the implicit entries
            if node_names.iter().any(|n| n == name) {
                return Err(err_at_node(desc, "cannot use reserved node name"));
            }

            if !nodes::is_valid_type(node_type) {
                return Err(err_at_node(desc, "unknown node type"));
            }

            ports.push(PortInfo::new(node_type, &unc.named_ins, &unc.named_outs));
        }

        for unc in &self.nodes {
            let name = &unc.desc.name;

            if node_names.contains(name) {
                return Err(format!("multiple definitions of node `{name}`"));
            }

            node_names.push(name.into());
        }

        let mut linked_inputs = vec![0; node_names.len()];
        for unc in self.nodes.iter_mut() {
            fixup_missing_port_names(unc, &node_names, &mut ports, &mut linked_inputs)
                .map_err(|e| err_at_node(&unc.desc, &e))?;
        }

        // Now that all user-given links are resolved,
        // we can create the user-given nodes
        // (which may add default links of their own into implicit nodes)
        for (u, unc) in self.nodes.iter_mut().enumerate() {
            nodes.push(make_node_info(unc, &ports[u + p]).map_err(|e| err_at_node(&unc.desc, &e))?);
        }

        let (input_names, output_names) = into_name_lists(ports);
        let mut graph = DependencyGraph::new(node_names, input_names, output_names);

        for unc in &self.nodes {
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
            node_list: nodes,
            graph,
            debug: self.debug,
        })
    }
}

impl Config {
    pub fn new(config_bytes: Vec<u8>, implicits: &[ImplicitNode]) -> Result<Config, String> {
        match de::from_slice::<UserConfig>(&config_bytes) {
            Ok(user_config) => user_config
                .into_config(implicits)
                .map_err(|err| format!("failed checking configuration: {err}")),
            Err(err) => Err(format!("failed parsing configuration: {err}")),
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

    pub fn get_node_name(&self, i: usize) -> &str {
        &self.node_list.get(i).expect("valid index").name
    }

    pub fn get_node_type(&self, i: usize) -> &str {
        &self.node_list.get(i).expect("valid index").node_type
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
    use crate::nodes::PortConfig;
    use serde_json::json;
    use std::any::Any;

    fn deserialize_user_config(cfg: &str) -> UserConfig {
        de::from_slice::<UserConfig>(cfg.as_bytes()).unwrap()
    }

    fn declare_implicits() -> Vec<ImplicitNode> {
        let req_ports: Vec<String> = PortConfig::names(&["body", "headers", "query"]);
        let resp_ports: Vec<String> = PortConfig::names(&["body", "headers"]);
        vec![
            ImplicitNode::new("request", vec![], req_ports.clone()),
            ImplicitNode::new("service_request", req_ports.clone(), resp_ports.clone()),
            ImplicitNode::new("service_response", vec![], resp_ports.clone()),
            ImplicitNode::new("response", resp_ports.clone(), resp_ports.clone()),
        ]
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
        nodes::register_node("implicit", Box::new(nodes::implicit::ImplicitFactory {}));
        let implicits = declare_implicits();

        let result = Config::new(cfg.as_bytes().to_vec(), &implicits);

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
            "failed checking configuration: in node `MY_NODE` of type `INVALID`: unknown node type",
        )
    }

    #[test]
    fn config_invalid_name() {
        nodes::register_node("jq", Box::new(nodes::jq::JqFactory {}));
        reject_config_with(
            r#"{
                "nodes": [
                    {
                        "name": "response",
                        "type": "jq"
                    }
                ]
            }"#,
            "failed checking configuration: in node `response` of type `jq`: cannot use reserved node name",
        )
    }

    #[test]
    fn config_invalid_loop() {
        nodes::register_node("jq", Box::new(nodes::jq::JqFactory {}));
        reject_config_with(
            r#"{
                "nodes": [
                    {
                        "name": "MY_NODE",
                        "type": "jq",
                        "inputs": {
                            "input": "MY_NODE"
                        }
                    }
                ]
            }"#,
            "failed checking configuration: in node `MY_NODE` of type `jq`: node cannot connect to itself",
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

        nodes::register_node("implicit", Box::new(nodes::implicit::ImplicitFactory {}));
        nodes::register_node("call", Box::new(nodes::call::CallFactory {}));
        nodes::register_node("jq", Box::new(nodes::jq::JqFactory {}));

        let implicits = declare_implicits();

        let config = uc.into_config(&implicits).unwrap();
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
                    name: "service_response".into(),
                    node_type: "implicit".into(),
                    node_config: Box::new(IgnoreConfig {}),
                },
                NodeInfo {
                    name: "response".into(),
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
            &[None, None, None],
            &[],
            &[None, None],
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
            &[&[(6, 1)], &[(4, 0)], &[]],
            &[&[], &[]],
            &[&[], &[]],
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
