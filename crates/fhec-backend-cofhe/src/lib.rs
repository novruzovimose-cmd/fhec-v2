mod call_graph;
mod cost_report;
mod emitter;
mod lowering;
mod optimizer;
mod type_map;

pub use call_graph::{FunctionCallGraphReport, FunctionInvocationCost};
pub use cost_report::{CoFheCostReport, FunctionCostReport, OperationProfile};
pub use emitter::{CoFheArtifact, CoFheError, CoFheOptions, compile_cofhe, emit_library};

#[cfg(test)]
mod call_graph_tests;
#[cfg(test)]
mod optimization_tests;
#[cfg(test)]
mod tests;
