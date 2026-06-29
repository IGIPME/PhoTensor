//! Binary GDSII loading.

use oxiphoton::geometry::gds::GdsLibrary;
use oxiphoton::io::gds_io::GdsBinaryReader;
use std::path::Path;

/// Loaded GDS library + the name of the cell we treat as the top of the layout.
#[derive(Debug, Clone)]
pub struct LoadedLibrary {
    pub lib: GdsLibrary,
    pub top_cell: String,
}

/// Errors surfaced by the loader.
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("failed to read GDS file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse GDS stream: {0}")]
    Parse(#[from] oxiphoton::error::OxiPhotonError),
    #[error("cell '{0}' not found in library")]
    CellNotFound(String),
}

/// Convenience handle over a loaded library + its top cell.
#[derive(Debug, Clone)]
pub struct Layout {
    pub loaded: LoadedLibrary,
}

impl Layout {
    /// Load a binary GDSII file, using `top_cell` as the layout root.
    pub fn from_path(path: &Path, top_cell: &str) -> Result<Self, LayoutError> {
        let lib = GdsBinaryReader::from_path(path)?;
        if !lib.cells.iter().any(|c| c.name == top_cell) {
            return Err(LayoutError::CellNotFound(top_cell.to_string()));
        }
        Ok(Self {
            loaded: LoadedLibrary {
                lib,
                top_cell: top_cell.to_string(),
            },
        })
    }

    /// Auto-detect the top cell: prefer `MATRIX_unit`, otherwise pick the cell
    /// that is never referenced by an SREF/AREF elsewhere in the library.
    pub fn from_path_autodetect(path: &Path) -> Result<Self, LayoutError> {
        let lib = GdsBinaryReader::from_path(path)?;
        let top = pick_top_cell(&lib).unwrap_or_else(|| "MATRIX_unit".to_string());
        if !lib.cells.iter().any(|c| c.name == top) {
            return Err(LayoutError::CellNotFound(top));
        }
        Ok(Self {
            loaded: LoadedLibrary { lib, top_cell: top },
        })
    }

    pub fn lib(&self) -> &GdsLibrary {
        &self.loaded.lib
    }

    pub fn top_cell(&self) -> &str {
        &self.loaded.top_cell
    }
}

fn pick_top_cell(lib: &GdsLibrary) -> Option<String> {
    use oxiphoton::geometry::gds::GdsElement;
    let mut referenced: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for cell in &lib.cells {
        for elem in &cell.elements {
            match elem {
                GdsElement::Sref(s) => {
                    referenced.insert(s.sname.as_str());
                }
                GdsElement::Aref(a) => {
                    referenced.insert(a.ref_name.as_str());
                }
                _ => {}
            }
        }
    }
    lib.cells
        .iter()
        .find(|c| !referenced.contains(c.name.as_str()))
        .map(|c| c.name.clone())
}
