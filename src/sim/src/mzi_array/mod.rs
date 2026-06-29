//! MZI mesh engine bridging GDS layouts to the Clements decomposition
//! provided by `oxiphoton::optical_computing::mzi_mesh`.

pub mod engine;

pub use engine::{DEFAULT_V_PER_RAD, EngineError, MziInstance, MziMatrixEngine, PortSet};
