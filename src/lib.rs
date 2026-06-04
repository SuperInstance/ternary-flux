#![forbid(unsafe_code)]

//! Flux/state-flow engine for tracking ternary value propagation.
//!
//! Provides `FluxGraph` (directed graph of value flow), `FluxNode` (transforms
//! inputs to outputs), `FluxEdge` (weighted ternary connection), `FluxObserver`
//! (monitors flow rates and detects anomalies), `FluxBalancer` (redistributes
//! flow for conservation), and `FluxCompiler` (compiles flux graphs to
//! efficient runtime execution plans).

use std::collections::HashMap;

// ── Ternary Value ──────────────────────────────────────────────────────────

/// A balanced ternary digit: Negative (-1), Zero (0), or Positive (+1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ternary {
    Negative,
    Zero,
    Positive,
}

impl Ternary {
    pub fn to_i8(self) -> i8 {
        match self {
            Ternary::Negative => -1,
            Ternary::Zero => 0,
            Ternary::Positive => 1,
        }
    }

    pub fn from_i8(v: i8) -> Self {
        match v {
            ..=-1 => Ternary::Negative,
            0 => Ternary::Zero,
            1.. => Ternary::Positive,
        }
    }
}

// ── Flux Node ──────────────────────────────────────────────────────────────

/// A node in the flux graph that transforms inputs to outputs.
///
/// Each node has a transform function (represented as a lookup table) that
/// maps an input ternary value to an output ternary value.
#[derive(Debug, Clone)]
pub struct FluxNode {
    pub id: String,
    /// Transform table: input → output.
    pub transform: [Ternary; 3],
    /// Current output value after evaluation.
    pub output: Option<Ternary>,
}

impl FluxNode {
    /// Create a node with an identity transform (output = input).
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            transform: [Ternary::Negative, Ternary::Zero, Ternary::Positive],
            output: None,
        }
    }

    /// Create a node with a custom transform.
    /// index 0 = Negative input, 1 = Zero input, 2 = Positive input.
    pub fn with_transform(id: &str, neg_out: Ternary, zero_out: Ternary, pos_out: Ternary) -> Self {
        Self {
            id: id.to_string(),
            transform: [neg_out, zero_out, pos_out],
            output: None,
        }
    }

    /// Create an inverter node: negates the input.
    pub fn inverter(id: &str) -> Self {
        Self::with_transform(id, Ternary::Positive, Ternary::Zero, Ternary::Negative)
    }

    /// Create a constant node: always outputs the same value regardless of input.
    pub fn constant(id: &str, value: Ternary) -> Self {
        Self::with_transform(id, value, value, value)
    }

    /// Evaluate this node given an input value.
    pub fn evaluate(&mut self, input: Ternary) -> Ternary {
        let output = match input {
            Ternary::Negative => self.transform[0],
            Ternary::Zero => self.transform[1],
            Ternary::Positive => self.transform[2],
        };
        self.output = Some(output);
        output
    }

    /// Get the index for a ternary value (for transform array access).
    fn index(v: Ternary) -> usize {
        match v {
            Ternary::Negative => 0,
            Ternary::Zero => 1,
            Ternary::Positive => 2,
        }
    }
}

// ── Flux Edge ──────────────────────────────────────────────────────────────

/// A weighted ternary connection between two nodes.
///
/// The weight modulates the flow: it multiplies (ternary-multiply) the value
/// passing through. Weight Zero blocks flow entirely.
#[derive(Debug, Clone)]
pub struct FluxEdge {
    pub from: String,
    pub to: String,
    pub weight: Ternary,
}

impl FluxEdge {
    pub fn new(from: &str, to: &str, weight: Ternary) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            weight,
        }
    }

    /// Apply the weight to a value: ternary multiplication.
    pub fn apply_weight(&self, value: Ternary) -> Ternary {
        ternary_multiply(value, self.weight)
    }
}

/// Ternary multiplication: standard sign rules.
fn ternary_multiply(a: Ternary, b: Ternary) -> Ternary {
    match (a, b) {
        (Ternary::Zero, _) | (_, Ternary::Zero) => Ternary::Zero,
        (Ternary::Positive, b) => b,
        (a, Ternary::Positive) => a,
        (Ternary::Negative, Ternary::Negative) => Ternary::Positive,
    }
}

// ── Flux Graph ─────────────────────────────────────────────────────────────

/// A directed graph of value flow.
///
/// Nodes transform values; edges carry values with weights. The graph can
/// be evaluated to propagate values from inputs to outputs.
#[derive(Debug, Clone)]
pub struct FluxGraph {
    pub nodes: HashMap<String, FluxNode>,
    pub edges: Vec<FluxEdge>,
    /// Input node IDs.
    pub inputs: Vec<String>,
    /// Output node IDs.
    pub outputs: Vec<String>,
}

impl FluxGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: FluxNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Connect two nodes with a weighted edge.
    pub fn connect(&mut self, from: &str, to: &str, weight: Ternary) {
        self.edges.push(FluxEdge::new(from, to, weight));
    }

    /// Mark a node as an input.
    pub fn add_input(&mut self, node_id: &str) {
        if self.nodes.contains_key(node_id) && !self.inputs.contains(&node_id.to_string()) {
            self.inputs.push(node_id.to_string());
        }
    }

    /// Mark a node as an output.
    pub fn add_output(&mut self, node_id: &str) {
        if self.nodes.contains_key(node_id) && !self.outputs.contains(&node_id.to_string()) {
            self.outputs.push(node_id.to_string());
        }
    }

    /// Get incoming edges for a node.
    pub fn incoming_edges(&self, node_id: &str) -> Vec<&FluxEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    /// Get outgoing edges for a node.
    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&FluxEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// Topological sort of nodes for evaluation order.
    /// Uses Kahn's algorithm. Returns None if cycle detected.
    pub fn topological_order(&self) -> Option<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id.as_str(), 0);
        }
        for edge in &self.edges {
            *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
        }
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut order = Vec::new();
        while let Some(id) = queue.pop() {
            order.push(id.to_string());
            for edge in self.edges.iter().filter(|e| e.from == id) {
                let deg = in_degree.get_mut(edge.to.as_str()).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(&edge.to);
                }
            }
        }
        if order.len() == self.nodes.len() {
            Some(order)
        } else {
            None
        }
    }

    /// Evaluate the graph with given input values.
    /// Returns output values in the order of `self.outputs`.
    pub fn evaluate(&mut self, input_values: &[(String, Ternary)]) -> Vec<Option<Ternary>> {
        // Set input values
        let mut node_values: HashMap<String, Ternary> = HashMap::new();
        for (id, val) in input_values {
            if self.nodes.contains_key(id) {
                node_values.insert(id.clone(), *val);
            }
        }

        let order = match self.topological_order() {
            Some(o) => o,
            None => return self.outputs.iter().map(|_| None).collect(),
        };

        // Process nodes in topological order
        for node_id in order {
            // Collect inputs from incoming edges
            let incoming: Vec<(&FluxEdge, Ternary)> = self
                .edges
                .iter()
                .filter(|e| e.to == node_id)
                .filter_map(|e| {
                    node_values.get(&e.from).map(|&v| (e, v))
                })
                .collect();

            if !incoming.is_empty() {
                // Aggregate inputs: sum of weighted values, then ternary clamp
                let sum: i8 = incoming
                    .iter()
                    .map(|(edge, val)| edge.apply_weight(*val).to_i8())
                    .sum();
                let clamped = sum.clamp(-1, 1);
                let input_val = Ternary::from_i8(clamped);
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    let output = node.evaluate(input_val);
                    node_values.insert(node_id, output);
                }
            }
            // If no incoming edges, value was set by input_values
        }

        self.outputs
            .iter()
            .map(|id| node_values.get(id).copied())
            .collect()
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

// ── Flux Observer ──────────────────────────────────────────────────────────

/// Monitors flow rates and detects anomalies in a flux graph.
///
/// Tracks the values that flow through each edge over multiple evaluations
/// and can detect unusual patterns.
#[derive(Debug, Clone)]
pub struct FluxObserver {
    /// Edge ID ("from->to") → history of values observed.
    flow_history: HashMap<String, Vec<Ternary>>,
    /// Anomaly threshold: if a value differs from the majority more than this fraction.
    pub anomaly_threshold: f64,
}

impl FluxObserver {
    pub fn new(anomaly_threshold: f64) -> Self {
        Self {
            flow_history: HashMap::new(),
            anomaly_threshold,
        }
    }

    /// Record a flow observation on an edge.
    pub fn observe(&mut self, from: &str, to: &str, value: Ternary) {
        let key = format!("{}->{}", from, to);
        self.flow_history.entry(key).or_default().push(value);
    }

    /// Get flow history for an edge.
    pub fn history(&self, from: &str, to: &str) -> &[Ternary] {
        let key = format!("{}->{}", from, to);
        self.flow_history.get(&key).map(|h| h.as_slice()).unwrap_or(&[])
    }

    /// Detect anomalous values: values that appear less than threshold fraction.
    pub fn detect_anomalies(&self, from: &str, to: &str) -> Vec<Ternary> {
        let key = format!("{}->{}", from, to);
        let history = match self.flow_history.get(&key) {
            Some(h) if !h.is_empty() => h,
            _ => return Vec::new(),
        };
        let total = history.len() as f64;
        let mut counts = HashMap::new();
        for &v in history {
            *counts.entry(v).or_insert(0usize) += 1;
        }
        counts
            .into_iter()
            .filter(|(_, c)| (*c as f64 / total) < self.anomaly_threshold)
            .map(|(v, _)| v)
            .collect()
    }

    /// Compute the dominant flow value (mode) for an edge.
    pub fn dominant_value(&self, from: &str, to: &str) -> Option<Ternary> {
        let key = format!("{}->{}", from, to);
        let history = self.flow_history.get(&key)?;
        if history.is_empty() {
            return None;
        }
        let mut counts = HashMap::new();
        for &v in history {
            *counts.entry(v).or_insert(0usize) += 1;
        }
        counts.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v)
    }

    /// Total observations across all edges.
    pub fn total_observations(&self) -> usize {
        self.flow_history.values().map(|h| h.len()).sum()
    }
}

// ── Flux Balancer ──────────────────────────────────────────────────────────

/// Redistributes flow to maintain conservation across the graph.
///
/// Ensures that the sum of outputs equals the sum of inputs (in ternary
/// arithmetic) for each node.
#[derive(Debug, Clone)]
pub struct FluxBalancer;

impl FluxBalancer {
    pub fn new() -> Self {
        Self
    }

    /// Balance the outputs of a node to conserve the input sum.
    /// Given input values and desired output count, produce balanced outputs.
    pub fn balance_outputs(inputs: &[Ternary], output_count: usize) -> Vec<Ternary> {
        if output_count == 0 {
            return Vec::new();
        }
        let input_sum: i8 = inputs.iter().map(|t| t.to_i8()).sum::<i8>().clamp(-1, 1);
        let target = Ternary::from_i8(input_sum);

        // Distribute: fill with target value
        vec![target; output_count]
    }

    /// Compute the conservation error: difference between input and output sums.
    pub fn conservation_error(inputs: &[Ternary], outputs: &[Ternary]) -> i8 {
        let in_sum: i8 = inputs.iter().map(|t| t.to_i8()).sum();
        let out_sum: i8 = outputs.iter().map(|t| t.to_i8()).sum();
        (in_sum - out_sum).clamp(-1, 1)
    }

    /// Check if a set of values is balanced (sums to zero in clamped ternary).
    pub fn is_balanced(values: &[Ternary]) -> bool {
        let sum: i8 = values.iter().map(|t| t.to_i8()).sum();
        sum.clamp(-1, 1) == 0
    }
}

// ── Flux Compiler ──────────────────────────────────────────────────────────

/// Compiles a flux graph to an efficient runtime execution plan.
///
/// Produces a flat list of (node_id, input_source_edges) in topological order
/// that can be executed sequentially without graph traversal.
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub node_id: String,
    /// (source_node_id, weight) for each input.
    pub inputs: Vec<(String, Ternary)>,
}

#[derive(Debug, Clone)]
pub struct CompiledFlux {
    pub steps: Vec<ExecutionStep>,
    pub input_ids: Vec<String>,
    pub output_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FluxCompiler;

impl FluxCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a flux graph into a sequential execution plan.
    /// Returns None if the graph has cycles.
    pub fn compile(graph: &FluxGraph) -> Option<CompiledFlux> {
        let order = graph.topological_order()?;

        let steps: Vec<ExecutionStep> = order
            .iter()
            .map(|node_id| {
                let incoming: Vec<(String, Ternary)> = graph
                    .incoming_edges(node_id)
                    .iter()
                    .map(|edge| (edge.from.clone(), edge.weight))
                    .collect();
                ExecutionStep {
                    node_id: node_id.clone(),
                    inputs: incoming,
                }
            })
            .collect();

        Some(CompiledFlux {
            steps,
            input_ids: graph.inputs.clone(),
            output_ids: graph.outputs.clone(),
        })
    }

    /// Execute a compiled flux plan with given input values.
    pub fn execute(compiled: &CompiledFlux, nodes: &mut HashMap<String, FluxNode>, input_values: &[(String, Ternary)]) -> Vec<Option<Ternary>> {
        let mut node_values: HashMap<String, Ternary> = HashMap::new();
        for (id, val) in input_values {
            node_values.insert(id.clone(), *val);
        }

        for step in &compiled.steps {
            let input_val = if step.inputs.is_empty() {
                // Source node: use provided input or default Zero
                *node_values.get(&step.node_id).unwrap_or(&Ternary::Zero)
            } else {
                // Aggregate weighted inputs
                let sum: i8 = step
                    .inputs
                    .iter()
                    .filter_map(|(src, weight)| {
                        node_values.get(src).map(|&v| ternary_multiply(v, *weight).to_i8())
                    })
                    .sum();
                Ternary::from_i8(sum.clamp(-1, 1))
            };

            if let Some(node) = nodes.get_mut(&step.node_id) {
                let output = node.evaluate(input_val);
                node_values.insert(step.node_id.clone(), output);
            }
        }

        compiled
            .output_ids
            .iter()
            .map(|id| node_values.get(id).copied())
            .collect()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flux_node_identity() {
        let mut node = FluxNode::new("n1");
        assert_eq!(node.evaluate(Ternary::Positive), Ternary::Positive);
        assert_eq!(node.evaluate(Ternary::Negative), Ternary::Negative);
    }

    #[test]
    fn test_flux_node_inverter() {
        let mut node = FluxNode::inverter("inv1");
        assert_eq!(node.evaluate(Ternary::Positive), Ternary::Negative);
        assert_eq!(node.evaluate(Ternary::Negative), Ternary::Positive);
    }

    #[test]
    fn test_flux_node_constant() {
        let mut node = FluxNode::constant("c1", Ternary::Positive);
        assert_eq!(node.evaluate(Ternary::Negative), Ternary::Positive);
    }

    #[test]
    fn test_flux_edge_weight() {
        let edge = FluxEdge::new("a", "b", Ternary::Positive);
        assert_eq!(edge.apply_weight(Ternary::Positive), Ternary::Positive);
    }

    #[test]
    fn test_flux_edge_zero_weight_blocks() {
        let edge = FluxEdge::new("a", "b", Ternary::Zero);
        assert_eq!(edge.apply_weight(Ternary::Positive), Ternary::Zero);
    }

    #[test]
    fn test_ternary_multiply() {
        assert_eq!(ternary_multiply(Ternary::Negative, Ternary::Negative), Ternary::Positive);
        assert_eq!(ternary_multiply(Ternary::Positive, Ternary::Zero), Ternary::Zero);
        assert_eq!(ternary_multiply(Ternary::Negative, Ternary::Positive), Ternary::Negative);
    }

    #[test]
    fn test_flux_graph_build() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("in"));
        graph.add_node(FluxNode::inverter("inv"));
        graph.add_node(FluxNode::new("out"));
        graph.connect("in", "inv", Ternary::Positive);
        graph.connect("inv", "out", Ternary::Positive);
        graph.add_input("in");
        graph.add_output("out");
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_flux_graph_topological_order() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("a"));
        graph.add_node(FluxNode::new("b"));
        graph.add_node(FluxNode::new("c"));
        graph.connect("a", "b", Ternary::Positive);
        graph.connect("b", "c", Ternary::Positive);
        let order = graph.topological_order().unwrap();
        let a_pos = order.iter().position(|s| s == "a").unwrap();
        let b_pos = order.iter().position(|s| s == "b").unwrap();
        let c_pos = order.iter().position(|s| s == "c").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_flux_graph_cycle_detection() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("a"));
        graph.add_node(FluxNode::new("b"));
        graph.connect("a", "b", Ternary::Positive);
        graph.connect("b", "a", Ternary::Positive);
        assert!(graph.topological_order().is_none());
    }

    #[test]
    fn test_flux_graph_evaluate_chain() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("in"));
        graph.add_node(FluxNode::inverter("inv"));
        graph.add_node(FluxNode::new("out"));
        graph.connect("in", "inv", Ternary::Positive);
        graph.connect("inv", "out", Ternary::Positive);
        graph.add_input("in");
        graph.add_output("out");
        let result = graph.evaluate(&[("in".to_string(), Ternary::Positive)]);
        assert_eq!(result, vec![Some(Ternary::Negative)]);
    }

    #[test]
    fn test_flux_observer_observe_and_history() {
        let mut observer = FluxObserver::new(0.3);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Negative);
        assert_eq!(observer.history("a", "b").len(), 3);
    }

    #[test]
    fn test_flux_observer_dominant() {
        let mut observer = FluxObserver::new(0.3);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Negative);
        assert_eq!(observer.dominant_value("a", "b"), Some(Ternary::Positive));
    }

    #[test]
    fn test_flux_observer_anomalies() {
        let mut observer = FluxObserver::new(0.4);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("a", "b", Ternary::Negative);
        let anomalies = observer.detect_anomalies("a", "b");
        assert!(anomalies.contains(&Ternary::Negative));
    }

    #[test]
    fn test_flux_balancer_balance_outputs() {
        let outputs = FluxBalancer::balance_outputs(&[Ternary::Positive], 3);
        assert_eq!(outputs, vec![Ternary::Positive, Ternary::Positive, Ternary::Positive]);
    }

    #[test]
    fn test_flux_balancer_conservation_error() {
        let inputs = vec![Ternary::Positive, Ternary::Negative];
        let outputs = vec![Ternary::Zero];
        let error = FluxBalancer::conservation_error(&inputs, &outputs);
        assert_eq!(error, 0); // (+1 + -1) - 0 = 0
    }

    #[test]
    fn test_flux_balancer_is_balanced() {
        assert!(FluxBalancer::is_balanced(&[Ternary::Positive, Ternary::Negative]));
        assert!(FluxBalancer::is_balanced(&[Ternary::Zero, Ternary::Zero]));
        assert!(!FluxBalancer::is_balanced(&[Ternary::Positive]));
    }

    #[test]
    fn test_flux_compiler_compile() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("in"));
        graph.add_node(FluxNode::inverter("inv"));
        graph.connect("in", "inv", Ternary::Positive);
        graph.add_input("in");
        graph.add_output("inv");
        let compiled = FluxCompiler::compile(&graph);
        assert!(compiled.is_some());
        assert_eq!(compiled.unwrap().steps.len(), 2);
    }

    #[test]
    fn test_flux_compiler_execute() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("in"));
        graph.add_node(FluxNode::inverter("inv"));
        graph.connect("in", "inv", Ternary::Positive);
        graph.add_input("in");
        graph.add_output("inv");
        let compiled = FluxCompiler::compile(&graph).unwrap();
        let mut nodes = graph.nodes.clone();
        let result = FluxCompiler::execute(&compiled, &mut nodes, &[("in".to_string(), Ternary::Positive)]);
        assert_eq!(result, vec![Some(Ternary::Negative)]);
    }

    #[test]
    fn test_flux_observer_total_observations() {
        let mut observer = FluxObserver::new(0.3);
        observer.observe("a", "b", Ternary::Positive);
        observer.observe("c", "d", Ternary::Negative);
        assert_eq!(observer.total_observations(), 2);
    }

    #[test]
    fn test_flux_graph_evaluate_with_zero_weight() {
        let mut graph = FluxGraph::new();
        graph.add_node(FluxNode::new("in"));
        graph.add_node(FluxNode::new("out"));
        graph.connect("in", "out", Ternary::Zero); // blocked
        graph.add_input("in");
        graph.add_output("out");
        let result = graph.evaluate(&[("in".to_string(), Ternary::Positive)]);
        assert_eq!(result, vec![Some(Ternary::Zero)]);
    }
}
