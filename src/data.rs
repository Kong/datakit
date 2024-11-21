use crate::dependency_graph::DependencyGraph;
use crate::payload::Payload;

#[allow(clippy::enum_variant_names)]
#[derive(PartialEq, Clone, Copy)]
pub enum Phase {
    HttpRequestHeaders,
    HttpRequestBody,
    HttpResponseHeaders,
    HttpResponseBody,
    HttpCallResponse,
}

pub struct Input<'a> {
    pub data: &'a [Option<&'a Payload>],
    pub phase: Phase,
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Waiting(u32),
    Done(Vec<Option<Payload>>),
    Fail(Vec<Option<Payload>>),
}

pub struct Data {
    graph: DependencyGraph,
    states: Vec<Option<State>>,
}

fn set_port(
    ports: &mut [Option<Payload>],
    port: usize,
    payload: Payload,
) -> Result<(), &'static str> {
    match &ports[port] {
        Some(_) => Err("cannot overwrite a payload"),
        None => {
            ports[port] = Some(payload);
            Ok(())
        }
    }
}

fn default_vec<T>(n: usize) -> Vec<T>
where
    T: Default,
{
    let mut vec = Vec::with_capacity(n);
    vec.resize_with(n, Default::default);
    vec
}

impl Data {
    pub fn new(graph: DependencyGraph) -> Data {
        let n = graph.number_of_nodes();
        let states = default_vec(n);
        Data { graph, states }
    }

    pub fn set(&mut self, node: usize, state: State) {
        self.states[node] = Some(state);
    }

    pub fn fill_port(
        &mut self,
        node: usize,
        port: usize,
        payload: Payload,
    ) -> Result<(), &'static str> {
        match &mut self.states[node] {
            None => {
                let mut ports: Vec<Option<Payload>> =
                    default_vec(self.graph.number_of_outputs(node));
                ports[port] = Some(payload);
                let state = State::Done(ports);
                self.states[node] = Some(state);
                Ok(())
            }
            Some(State::Waiting(_)) => Err("cannot force payload on a waiting node"),
            Some(State::Done(ports)) => set_port(ports, port, payload),
            Some(State::Fail(ports)) => set_port(ports, port, payload),
        }
    }

    pub fn get_state(&self, node: usize) -> Result<&State, &'static str> {
        match &self.states[node] {
            None => Err("fill_port must have created a state"),
            Some(state) => Ok(state),
        }
    }

    pub fn fetch_port(&self, node: usize, port: usize) -> Option<&Payload> {
        match self.graph.get_provider(node, port) {
            Some((n, p)) => match self.states.get(n).unwrap() {
                Some(State::Waiting(_)) => None,
                Some(State::Done(ports)) | Some(State::Fail(ports)) => match ports.get(p) {
                    Some(Some(ref payload)) => Some(payload),
                    Some(None) => None,
                    None => None,
                },
                None => None,
            },
            None => None,
        }
    }

    fn can_trigger(&self, i: usize, waiting: Option<u32>) -> bool {
        // This is intentionally written with all of the match arms
        // stated explicitly (instead of using _ catch-alls),
        // so that the trigger logic and all its states
        // are considered by the reader.
        match &self.states[i] {
            // state was never created, trigger
            None => true,
            Some(state) => match state {
                // never retrigger Done
                State::Done(_) => false,
                // never retrigger Fail
                State::Fail(_) => false,
                State::Waiting(w) => match &waiting {
                    // we're waiting on the right id, allow triggering
                    Some(id) if w == id => true,
                    // waiting on something else, skip
                    Some(_) => false,
                    // not called from a wait state
                    None => false,
                },
            },
        }
    }

    fn for_each_input<'a, T>(
        &'a self,
        i: usize,
        f: impl for<'b> Fn(Option<&'a Payload>, &'b mut T),
        mut t: T,
    ) -> Option<T> {
        for input in self.graph.each_input(i) {
            // if input port is connected in the graph
            match *input {
                Some((n, p)) => {
                    // check if other node is Done
                    match &self.states[n] {
                        Some(State::Done(ports)) => {
                            // check if port has payload available
                            match &ports[p] {
                                // ok, has payload
                                Some(payload) => f(Some(payload), &mut t),
                                // no payload available
                                None => return None,
                            }
                        }
                        Some(State::Waiting(_)) => return None,
                        Some(State::Fail(_)) => return None,
                        None => return None,
                    }
                }
                None => f(None, &mut t), // ok, port is not connected
            }
        }

        Some(t)
    }

    pub fn get_inputs_for(
        &self,
        node: usize,
        waiting: Option<u32>,
    ) -> Option<Vec<Option<&Payload>>> {
        if !self.can_trigger(node, waiting) {
            return None;
        }

        // Check first that all connected inputs are ready
        self.for_each_input(node, |_, _| (), &mut ())?;

        // If so, allocate the vector with the result.
        let n = self.graph.number_of_inputs(node);
        self.for_each_input(
            node,
            |payload, v: &mut Vec<Option<&Payload>>| match payload {
                Some(p) => v.push(Some(p)),
                None => v.push(None),
            },
            Vec::with_capacity(n),
        )
    }
}
