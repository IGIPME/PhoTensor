//! GDSII loading and topology extraction for the PhoTensor simulator.
//!
//! Wraps `oxiphoton`'s binary GDSII reader and adds a thin layer that flattens
//! the cell hierarchy (SREF/AREF with rotation, magnification, x-reflection
//! and array replication) so the rest of the simulator can work in world
//! coordinates.

mod flatten;
mod loader;

pub use flatten::{FlatLabel, FlatRef, Flattened, PlacementKind};
pub use loader::{Layout, LayoutError, LoadedLibrary};
