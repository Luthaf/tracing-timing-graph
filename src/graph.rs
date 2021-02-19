use petgraph::dot::Dot;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Direction;

use term_table::row::Row;
use term_table::table_cell::{Alignment, TableCell};

use std::time::Duration;

/// Data associated with a set of span sharing the same name.
///
/// All spans wih the same name are grouped together. The full span name is
/// constructed with the `tracing::Span` name and associated module path, or
/// span target if the module path is not provided.
///
/// The main intent of this is to group multiple calls to the same function
/// together.
#[derive(Clone, Debug)]
pub struct SpanTiming {
    /// Span identifier, monotonically increasing
    pub id: usize,
    /// Full span name, including the module path or span target
    pub name: String,
    /// Total elapsed time in all spans sharing this name
    pub elapsed: Duration,
    /// Number of time a span with this name have been called
    pub called: usize,
}

impl std::fmt::Display for SpanTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ran for {:?}, called {} times",
            self.name, self.elapsed, self.called
        )
    }
}

impl SpanTiming {
    pub(crate) fn new(name: String, id: usize) -> SpanTiming {
        SpanTiming {
            id: id,
            name: name,
            elapsed: Duration::new(0, 0),
            called: 0,
        }
    }
}

/// Full span graph including execution time and number of calls
///
/// The span graph is a directed graph linking different `SpanTiming` by the
/// number of time a given span was the child of another one.
///
/// For example, code like this:
/// ```no_run
/// #[tracing::instrument]
/// fn start() {
///     inside();
///     inside();
///     inner();
/// }
///
/// #[tracing::instrument]
/// fn inside() {
///    inner();
/// }
///
/// #[tracing::instrument]
/// fn inner() {
///     // do stuff
/// }
/// ```
///
/// Will result in a graph like this, where the number near the edge correspond
/// to the number of time a given span called another one.
/// ```bash no_run
///             | start, called 1 |
///                /           |
///              /  2          |
///            /               |  1
///   | inside, called 2 |     |
///                 \          |
///                 2 \        |
///                     \      |
///                  | inner, called 3 |
/// ```
pub struct SpanGraph {
    graph: Graph<SpanTiming, usize>,
    last_id: usize,
}

/// A set of calls from one span to another
pub struct Calls {
    /// the outer/calling span/function
    pub caller: SpanIndex,
    /// the inner/called span/function
    pub callee: SpanIndex,
    /// number of time the inner span/function have been called by the outer one
    pub count: usize,
}

/// Opaque span identifier inside a `SpanGraph`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanIndex(usize);

impl From<NodeIndex> for SpanIndex {
    fn from(index: NodeIndex) -> SpanIndex {
        SpanIndex(index.index())
    }
}

impl From<SpanIndex> for NodeIndex {
    fn from(index: SpanIndex) -> NodeIndex {
        NodeIndex::new(index.0)
    }
}

impl SpanGraph {
    /// Create a new empty `SpanGraph`
    pub fn new() -> SpanGraph {
        SpanGraph {
            graph: Graph::new(),
            last_id: 0,
        }
    }

    /// Find a span in the graph, given its name
    pub fn find(&self, name: &str) -> Option<SpanIndex> {
        for id in self.graph.node_indices() {
            if self.graph[id].name == name {
                return Some(id.into());
            }
        }
        return None;
    }

    /// Find a span in the graph given its name, or create a new empty span
    /// with the given name
    pub fn find_or_create(&mut self, name: &str) -> SpanIndex {
        match self.find(name) {
            Some(node) => node,
            None => {
                // could not find the node, add a new one
                let node_id = self
                    .graph
                    .add_node(SpanTiming::new(name.into(), self.last_id));
                self.last_id += 1;
                node_id.into()
            }
        }
    }

    /// Increase the timing associated with a span by `time`, and the number of
    /// time this span has been called by one.
    pub fn increase_timing(&mut self, span: SpanIndex, time: Duration) {
        let id = NodeIndex::from(span);
        self.graph[id].elapsed += time;
        self.graph[id].called += 1;
    }

    /// Increase the number of time the `parent` span called the `child` span
    /// by one.
    pub fn increase_call_count(&mut self, parent: SpanIndex, child: SpanIndex) {
        let parent = NodeIndex::from(parent);
        let child = NodeIndex::from(child);
        if let Some(edge) = self.graph.find_edge(parent, child) {
            let count = self
                .graph
                .edge_weight_mut(edge)
                .expect("failed to get edge weights");
            *count += 1;
        } else {
            // initialize edge count to 1
            self.graph.add_edge(parent, child, 1);
        }
    }

    /// Get a single span knowing its `SpanIndex`
    pub fn span(&self, id: SpanIndex) -> &SpanTiming {
        &self.graph[NodeIndex::from(id)]
    }

    /// Get the full list of spans known by this graph
    pub fn spans(&self) -> impl Iterator<Item = &SpanTiming> {
        self.graph.raw_nodes().iter().map(|node| &node.weight)
    }

    /// Get the list of calls between spans in this graph
    pub fn calls(&self) -> impl Iterator<Item = Calls> + '_ {
        self.graph.raw_edges().iter().map(|edge| Calls {
            caller: edge.target().into(),
            callee: edge.source().into(),
            count: edge.weight,
        })
    }

    /// Get the full graph in [graphviz](https://graphviz.org/) dot format.
    ///
    /// The exact output is unstable and should not be relied on.
    pub fn as_dot(&self) -> String {
        Dot::new(&self.graph).to_string()
    }

    /// Get a per span summary table of this graph.
    ///
    /// The exact output is unstable and should not be relied on.
    ///
    /// # Panic
    ///
    /// This function will panic if the graph is cyclical, i.e. if two or more
    /// span are mutually recursive.
    pub fn as_table(&self) -> String {
        let mut table = term_table::Table::new();
        table.style = term_table::TableStyle::extended();

        table.add_row(Row::new(vec![
            "id",
            // pad "span name" to make the table look nicer with short names
            "span name                                   ",
            "call count",
            "called by",
            "duration",
        ]));

        for &node_id in petgraph::algo::kosaraju_scc(&self.graph)
            .iter()
            .rev()
            .flatten()
        {
            let data = &self.graph[node_id];

            let mut called_by = vec![];
            for other in self.graph.neighbors_directed(node_id, Direction::Incoming) {
                called_by.push(self.graph[other].id.to_string());
            }
            let called_by = if !called_by.is_empty() {
                called_by.join(", ")
            } else {
                "—".into()
            };

            table.add_row(Row::new(vec![
                TableCell::new_with_alignment(self.graph[node_id].id, 1, Alignment::Right),
                TableCell::new(&data.name),
                TableCell::new_with_alignment(data.called, 1, Alignment::Right),
                TableCell::new_with_alignment(called_by, 1, Alignment::Right),
                TableCell::new_with_alignment(
                    &format!("{:.2?}", data.elapsed),
                    1,
                    Alignment::Right,
                ),
            ]));
        }

        return table.render();
    }

    /// Get all the data in this graph in JSON.
    ///
    /// The exact output is unstable and should not be relied on.
    pub fn as_json(&self) -> String {
        let mut spans = json::JsonValue::new_object();
        for span in self.spans() {
            spans[&span.name] = json::object! {
                "id" => span.id,
                "elapsed" => format!("{} µs", span.elapsed.as_micros()),
                "called" => span.called,
            };
        }

        let mut all_calls = json::JsonValue::new_array();
        for call in self.calls() {
            all_calls
                .push(json::object! {
                    "caller" => self.span(call.caller).id,
                    "callee" => self.span(call.caller).id,
                    "count" => call.count,
                })
                .expect("failed to add edge information to JSON");
        }

        return json::stringify(json::object! {
            "timings" => spans,
            "calls" => all_calls,
        });
    }

    pub fn clear(&mut self) {
        self.graph.clear();
        self.last_id = 0;
    }
}
