use pyo3::prelude::*;

mod graph;

pub use graph::{Edge, EdgeType, Graph, Vertex, VertexStatus};

/// Python module exported by maturin.
#[pymodule]
fn graph_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VertexStatus>()?;
    m.add_class::<EdgeType>()?;
    m.add_class::<Vertex>()?;
    m.add_class::<Edge>()?;
    m.add_class::<Graph>()?;
    Ok(())
}
