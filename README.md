# ternary-flux: Flux/state-flow engine for tracking ternary value propagation

## Why This Exists

Values in SuperInstance don't just sit still — they flow. A sensor reading propagates through transforms, gets weighted by connections, and eventually produces an output. Without a structured flow engine, this propagation is ad-hoc string-pulling that's impossible to debug or reason about. This crate provides the directed graph, nodes, edges, and runtime that make room physics actually run.

## Core Concepts

**FluxGraph**: A directed graph where nodes transform ternary values and edges carry them with weights. Think of it as a circuit diagram for ternary logic.

**FluxNode**: A processing element with a transform table. Given an input ternary value, it produces an output. Special cases: identity (output = input), inverter (negates), constant (always outputs the same value).

**FluxEdge**: A weighted connection. The weight uses ternary multiplication: Positive weight passes the value through, Negative weight negates it, Zero weight blocks it entirely. This makes Zero-weighted edges act as switches.

**FluxObserver**: Watches flow over time, records history, and detects anomalies — values that appear less frequently than a threshold.

**FluxBalancer**: Ensures conservation: the sum of outputs should equal the sum of inputs (in clamped ternary arithmetic).

**FluxCompiler**: Flattens the graph into a sequential execution plan (topological order) that can be evaluated without graph traversal on each step.

**Ternary multiplication**: The standard sign rules. Positive × x = x, Negative × x = -x, Zero × anything = Zero. This is what makes weighted edges work.

## Quick Start

```toml
[dependencies]
ternary-flux = "0.1"
```

```rust
use ternary_flux::{FluxGraph, FluxNode, Ternary, FluxCompiler};

// Build a graph: input → inverter → output
let mut graph = FluxGraph::new();
graph.add_node(FluxNode::new("input"));
graph.add_node(FluxNode::inverter("inv"));
graph.add_node(FluxNode::new("output"));
graph.connect("input", "inv", Ternary::Positive);
graph.connect("inv", "output", Ternary::Positive);
graph.add_input("input");
graph.add_output("output");

// Evaluate: Positive input becomes Negative after inversion
let result = graph.evaluate(&[("input".to_string(), Ternary::Positive)]);
assert_eq!(result, vec![Some(Ternary::Negative)]);

// Or compile for repeated evaluation
let compiled = FluxCompiler::compile(&graph).unwrap();
let mut nodes = graph.nodes.clone();
let result = FluxCompiler::execute(&compiled, &mut nodes,
    &[("input".to_string(), Ternary::Positive)]);
assert_eq!(result, vec![Some(Ternary::Negative)]);
```

## API Overview

| Type | What it is |
|------|-----------|
| `FluxGraph` | Directed graph of nodes and weighted edges |
| `FluxNode` | Transform node with lookup table (identity, inverter, constant) |
| `FluxEdge` | Weighted connection using ternary multiplication |
| `FluxObserver` | Records flow history and detects anomalous values |
| `FluxBalancer` | Redistributes outputs for conservation |
| `FluxCompiler` | Compiles graph to sequential execution plan |
| `CompiledFlux` | Flattened execution plan with ordered steps |
| `Ternary` | The three values: Negative, Zero, Positive |

## How It Works

Evaluation uses topological sorting (Kahn's algorithm) to process nodes in dependency order. For each node, incoming values are collected from edges, weighted by ternary multiplication, summed, and clamped to {-1, 0, +1}. This aggregated input is then transformed by the node's lookup table.

The FluxCompiler pre-computes the topological order and edge structure into a flat `Vec<ExecutionStep>`. Each step contains the node ID and its input sources. Execution is then a simple loop — no graph traversal needed. This is significantly faster for repeated evaluation of the same graph structure.

Cycle detection is built into the topological sort: if the sorted order contains fewer nodes than the graph, a cycle exists and compilation/evaluation fails gracefully.

The FluxObserver maintains a per-edge history of ternary values. Anomaly detection finds values whose frequency falls below a threshold fraction — useful for detecting when a normally-stable flow suddenly produces unusual values.

## Known Limitations

- **Aggregation is lossy**: When multiple inputs arrive at a node, they're summed and clamped to {-1, 0, +1}. A Positive + Negative input cancels to Zero, losing information about both inputs. This is inherent to ternary arithmetic.
- **No backpropagation**: Values flow forward only. There's no mechanism to push corrections backward through the graph.
- **Single-valued edges**: Each edge carries exactly one ternary value per evaluation. Multi-valued or streaming flows aren't supported.
- **Observer history is unbounded**: Flow history grows without limit. Long-running systems should periodically clear or trim history.
- **No parallel evaluation**: The compiled plan is strictly sequential. Independent subgraphs could theoretically run in parallel, but this isn't implemented.

## Use Cases

- **Sensor processing pipeline**: Sensor readings flow through filters, transforms, and aggregators to produce control signals.
- **Decision propagation**: A captain's ternary decision flows through a hierarchy, weighted by trust and distance.
- **Room physics**: Physical properties (temperature, occupancy, light) flow between rooms through weighted connections.
- **Anomaly detection**: Monitor flow patterns to detect when a room's outputs deviate from historical norms.
- **Signal routing**: Zero-weighted edges act as switches to enable/disable paths in the flow graph.

## Ecosystem Context

Part of the SuperInstance ternary ecosystem. This is the execution engine:

- `ternary-agent` agent outputs feed into FluxGraph inputs
- `ternary-captain` decisions flow through the graph to subordinates
- `ternary-muse` generated patterns can be injected as input values
- `ternary-tidelight` timing determines when graphs are evaluated

No external dependencies — pure Rust.

## License

MIT
