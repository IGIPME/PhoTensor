//! Pure-software photonic neural network simulator.
//!
//! Currently focuses on loading MZI mesh layouts from GDSII and driving them
//! through the Clements-decomposition engine provided by `oxiphoton`.

pub mod gds;
pub mod mzi_array;

pub use gds::Layout;
