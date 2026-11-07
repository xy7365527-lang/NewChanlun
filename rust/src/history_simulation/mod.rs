//! #1381：X 内已知前沿 bar 批模型。
pub(crate) mod intake;
pub mod model;
#[cfg(unix)]
pub mod runtime;
pub mod store;
#[cfg(test)]
pub(crate) mod tests;

mod capacity;
