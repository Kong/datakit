#[derive(Clone, PartialEq, Debug)]
pub struct DependencyGraph {
    node_names: Vec<String>,
    input_names: Vec<Vec<String>>,
    output_names: Vec<Vec<String>>,
    dependents: Vec<Vec<Vec<(usize, usize)>>>,
    providers: Vec<Vec<Option<(usize, usize)>>>,
}

pub fn find(
    node: &str,
    port: &str,
    node_names: &[String],
    port_names: &[Vec<String>],
) -> (usize, usize) {
    let n = node_names
        .iter()
        .position(|x| x == node)
        .expect("node registered in node_names");
    let p = port_names
        .get(n)
        .expect("valid node index")
        .iter()
        .position(|x| x == port)
        .expect("port registered in port_names");
    (n, p)
}

impl DependencyGraph {
    pub fn new(
        node_names: Vec<String>,
        input_names: Vec<Vec<String>>,
        output_names: Vec<Vec<String>>,
    ) -> DependencyGraph {
        let n = node_names.len();
        let mut dependents = Vec::with_capacity(n);
        let mut providers = Vec::with_capacity(n);
        for ports in &input_names {
            providers.push(vec![None; ports.len()]);
        }
        for ports in &output_names {
            let np = ports.len();
            let mut lists = Vec::with_capacity(np);
            lists.resize_with(np, Vec::new);
            dependents.push(lists);
        }
        DependencyGraph {
            node_names,
            input_names,
            output_names,
            dependents,
            providers,
        }
    }

    pub fn get_node_name(&self, i: usize) -> Option<&str> {
        self.node_names.get(i).map(|o| o.as_ref())
    }

    pub fn number_of_nodes(&self) -> usize {
        self.node_names.len()
    }

    pub fn number_of_outputs(&self, node: usize) -> usize {
        self.output_names[node].len()
    }

    pub fn number_of_inputs(&self, node: usize) -> usize {
        self.input_names[node].len()
    }

    fn add_dependent(&mut self, node: usize, port: usize, entry: (usize, usize)) {
        let node_list = &mut self.dependents;
        let port_list = node_list.get_mut(node).expect("valid node index");
        let entries_list = port_list.get_mut(port).expect("valid port index");
        entries_list.push(entry);
    }

    fn add_provider(
        &mut self,
        node: usize,
        port: usize,
        entry: (usize, usize),
    ) -> Result<(), String> {
        let node_list = &mut self.providers;
        let port_list = node_list.get_mut(node).expect("valid node index");
        match *port_list.get(port).expect("valid port index") {
            Some((other_n, other_p)) => self.err_already_connected(node, port, other_n, other_p),
            None => {
                port_list[port] = Some(entry);
                Ok(())
            }
        }
    }

    fn err_already_connected(
        &self,
        n: usize,
        p: usize,
        oth_n: usize,
        oth_p: usize,
    ) -> Result<(), String> {
        let this_node = self.node_names.get(n).expect("valid node");
        let this_port = self.output_names[n].get(p).expect("valid port");
        let other_node = self.node_names.get(oth_n).expect("valid node");
        let other_port = self.output_names[oth_n].get(oth_p).expect("valid port");
        Err(format!(
            "{this_node}.{this_port} is already connected to {other_node}.{other_port}"
        ))
    }

    pub fn add(
        &mut self,
        src_node: &str,
        src_port: &str,
        dst_node: &str,
        dst_port: &str,
    ) -> Result<(), String> {
        let (sn, sp) = find(src_node, src_port, &self.node_names, &self.output_names);
        let (dn, dp) = find(dst_node, dst_port, &self.node_names, &self.input_names);
        self.add_dependent(sn, sp, (dn, dp));
        self.add_provider(dn, dp, (sn, sp))
    }

    pub fn has_dependents(&self, node: usize, port: usize) -> bool {
        !self.dependents[node][port].is_empty()
    }

    pub fn has_providers(&self, node: usize, port: usize) -> bool {
        self.providers[node][port].is_some()
    }

    pub fn get_provider(&self, node: usize, port: usize) -> Option<(usize, usize)> {
        self.providers[node][port]
    }

    pub fn each_input(&self, node: usize) -> std::slice::Iter<Option<(usize, usize)>> {
        self.providers[node].iter()
    }

    /// used in tests only
    #[allow(dead_code)]
    pub fn each_output(&self, node: usize) -> std::slice::Iter<Vec<(usize, usize)>> {
        self.dependents[node].iter()
    }
}
