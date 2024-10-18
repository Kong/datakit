use std::collections::BTreeMap;

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

#[derive(Debug)]
pub enum State {
    Waiting(u32),
    Done(Option<Payload>),
    Fail(Option<Payload>),
}

#[derive(Default)]
pub struct Data {
    graph: DependencyGraph,
    states: BTreeMap<String, State>,
}

impl Data {
    pub fn new(graph: DependencyGraph) -> Data {
        Data {
            graph,
            states: Default::default(),
        }
    }

    pub fn set(&mut self, name: &str, state: State) {
        self.states.insert(name.to_string(), state);
    }

    fn can_trigger(&self, name: &str, waiting: Option<u32>) -> bool {
        // If node is Done, avoid producing inputs
        // and re-triggering its execution.
        if let Some(state) = self.states.get(name) {
            match state {
                State::Done(_) => {
                    return false;
                }
                State::Waiting(w) => match &waiting {
                    Some(id) => {
                        if w != id {
                            return false;
                        }
                    }
                    None => return false,
                },
                State::Fail(_) => {
                    return false;
                }
            }
        }

        // Check that all inputs have payloads available
        for input in self.graph.each_input(name) {
            let val = self.states.get(input);
            match val {
                Some(State::Done(_)) => {}
                _ => {
                    return false;
                }
            };
        }

        true
    }

    pub fn get_inputs_for(
        &self,
        name: &str,
        waiting: Option<u32>,
    ) -> Option<Vec<Option<&Payload>>> {
        if !self.can_trigger(name, waiting) {
            return None;
        }

        // If so, allocate the vector with the result.
        let mut vec: Vec<Option<&Payload>> = Vec::new();
        for input in self.graph.each_input(name) {
            if let Some(State::Done(p)) = self.states.get(input) {
                vec.push(p.as_ref());
            }
        }

        Some(vec)
    }

    /// If the node is triggerable, that is, it has all its required
    /// inputs available to trigger (i.e. none of its inputs are in a
    /// `Waiting` state), then return the payload of the first input that
    /// is in a `Done state.
    ///
    /// Note that by returning an `Option<&Payload>` this makes no
    /// distinction between the node being not triggerable or the
    /// node being triggerable via a `Done(None)` input.
    ///
    /// This is not an issue because this function is intended for use
    /// with the implicit nodes (`response_body`, etc.) which are
    /// handled as special cases directly by the filter.
    pub fn first_input_for(&self, name: &str, waiting: Option<u32>) -> Option<&Payload> {
        if !self.can_trigger(name, waiting) {
            return None;
        }

        for input in self.graph.each_input(name) {
            if let Some(State::Done(p)) = self.states.get(input) {
                return p.as_ref();
            }
        }

        None
    }
}
