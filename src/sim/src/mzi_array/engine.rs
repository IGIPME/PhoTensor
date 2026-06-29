//! MZI mesh engine: bridge between a GDS-loaded layout and `oxiphoton`'s
//! Clements-decomposition mesh.
//!
//! The engine:
//!   1. Takes a `Flattened` layout whose top cell holds 9 `MZI` placements and
//!      a global electrode label set `ep_1..ep_{4*n_mzi}`.
//!   2. Builds an `n`-port `ClementsArch` (n = max(n_inputs, n_outputs)).
//!   3. Maps the global electrodes onto MZI phase shifters in mesh order:
//!      for MZI index k, `ep_{4k+1}` → θ, `ep_{4k+2}` → φ (the other two are
//!      treated as mirror/ground and unused by the linear-phase model for now).
//!   4. Exposes:
//!        - `apply(input)` → optical matrix-vector product
//!        - `program(unitary)` → Clements decomposition
//!        - `set_electrode(ep_idx, voltage)` → linear voltage-to-phase (V·K)
//!        - `set_phase(ep_idx, theta_or_phi)` → direct phase setter

use oxiphoton::optical_computing::mzi_mesh::{ClementsArch, MziCell};
use std::path::Path;

use crate::gds::{FlatRef, Flattened, Layout, LayoutError};

/// Default linear coefficient mapping volts → radians (placeholder).
/// A real TiN thermo-optic phase shifter on SOI typically gives ~π/V·cm; this
/// constant is intentionally left as a tunable knob, not a calibrated value.
pub const DEFAULT_V_PER_RAD: f64 = 1.0;

/// Errors raised by the engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
    #[error("expected MZI placements in the top cell, found {0}")]
    NoMzis(usize),
    #[error("electrode index {0} out of range (max {1})")]
    ElectrodeOutOfRange(usize, usize),
    #[error("input length {0} does not match mesh port count {1}")]
    InputLength(usize, usize),
}

/// One MZI instance extracted from the GDS, in mesh order.
#[derive(Debug, Clone)]
pub struct MziInstance {
    /// 0-based index in the mesh (assigned by spatial column-major sort).
    pub mesh_index: usize,
    /// World-coordinate origin (DB units) of the MZI placement.
    pub origin_x: i64,
    pub origin_y: i64,
    /// Whether the underlying SREF mirrored the cell about x.
    pub x_reflect: bool,
    /// Four global electrode indices assigned to this MZI, in GDS label order.
    /// `electrodes[0]` → θ, `electrodes[1]` → φ, the rest are reserved.
    pub electrodes: [usize; 4],
}

/// Port set extracted from the top cell's TEXT labels.
#[derive(Debug, Clone)]
pub struct PortSet {
    /// Left (input) port labels sorted by index.
    pub inputs: Vec<String>,
    /// Right (output) port labels sorted by index.
    pub outputs: Vec<String>,
}

/// The MZI mesh engine.
pub struct MziMatrixEngine {
    pub n_ports: usize,
    pub arch: ClementsArch,
    pub instances: Vec<MziInstance>,
    pub ports: PortSet,
    /// Volts → radians coefficient for `set_electrode`.
    pub v_per_rad: f64,
}

impl MziMatrixEngine {
    /// Build the engine from a GDS file and the top cell name.
    pub fn from_gds(path: &Path, top_cell: &str) -> Result<Self, EngineError> {
        let layout = Layout::from_path(path, top_cell)?;
        Self::from_layout(&layout)
    }

    /// Build the engine from an already-loaded layout.
    pub fn from_layout(layout: &Layout) -> Result<Self, EngineError> {
        let flat = Flattened::from_layout(layout);
        Self::from_flattened(&flat)
    }

    /// Construct from a flattened view. Kept pub so callers can inspect the
    /// flattening before paying for engine construction.
    pub fn from_flattened(flat: &Flattened) -> Result<Self, EngineError> {
        // Collect top-level MZI placements, sorted column-major (x then y).
        let mut mzis: Vec<&FlatRef> = flat.top_level_instances("MZI").collect();
        if mzis.is_empty() {
            return Err(EngineError::NoMzis(0));
        }
        mzis.sort_by(|a, b| {
            a.xform
                .tx
                .cmp(&b.xform.tx)
                .then(a.xform.ty.cmp(&b.xform.ty))
        });

        let _n_mzi = mzis.len();
        // Each MZI is assigned 4 global electrodes in mesh order.
        // electrode[k*2]   -> theta
        // electrode[k*2+1] -> phi
        // (the other 2 of the 4 are reserved/ground in this model)
        let instances: Vec<MziInstance> = mzis
            .iter()
            .enumerate()
            .map(|(i, r)| MziInstance {
                mesh_index: i,
                origin_x: r.xform.tx,
                origin_y: r.xform.ty,
                x_reflect: r.xform.x_reflect,
                electrodes: [4 * i + 1, 4 * i + 2, 4 * i + 3, 4 * i + 4],
            })
            .collect();

        // Ports: `op_l_*` are inputs, `op_r_*` are outputs.
        let inputs = sorted_labels_with_prefix(flat, "op_l_");
        let outputs = sorted_labels_with_prefix(flat, "op_r_");
        let n_ports = inputs.len().max(outputs.len()).max(2);

        let arch = ClementsArch::new(n_ports);

        Ok(Self {
            n_ports,
            arch,
            instances,
            ports: PortSet { inputs, outputs },
            v_per_rad: DEFAULT_V_PER_RAD,
        })
    }

    /// Number of MZI instances loaded from the layout.
    pub fn n_mzis(&self) -> usize {
        self.instances.len()
    }

    /// Number of global electrodes tracked by the engine (4 per MZI).
    pub fn n_electrodes(&self) -> usize {
        self.instances.len() * 4
    }

    /// Apply the mesh to an input vector (length must equal `n_ports`).
    pub fn apply(
        &self,
        input: &[num_complex::Complex64],
    ) -> Result<Vec<num_complex::Complex64>, EngineError> {
        if input.len() != self.n_ports {
            return Err(EngineError::InputLength(input.len(), self.n_ports));
        }
        Ok(self.arch.apply(input))
    }

    /// Program the mesh to realize a target unitary (Clements decomposition).
    pub fn program(&mut self, target: &[Vec<num_complex::Complex64>]) {
        self.arch.program(target);
    }

    /// Return the current effective unitary realized by the mesh.
    pub fn to_unitary(&self) -> Vec<Vec<num_complex::Complex64>> {
        self.arch.to_unitary()
    }

    /// Set a phase shifter directly by global electrode index.
    ///
    /// Electrodes with an odd index (1-based: 1,5,9,...) drive θ of their MZI;
    /// electrodes with an even index drive φ. Other indices (the 2 reserved
    /// per MZI) are accepted but currently no-op.
    pub fn set_phase(&mut self, ep_idx_1based: usize, phase_rad: f64) -> Result<(), EngineError> {
        let max_ep = self.n_electrodes();
        if ep_idx_1based == 0 || ep_idx_1based > max_ep {
            return Err(EngineError::ElectrodeOutOfRange(ep_idx_1based, max_ep));
        }
        let zero_based = ep_idx_1based - 1;
        let mzi_idx = zero_based / 4;
        let role = zero_based % 4; // 0 -> theta, 1 -> phi, 2,3 reserved
        let Some(cell) = self.cell_at(mzi_idx) else {
            return Ok(());
        };
        match role {
            0 => cell.theta = phase_rad,
            1 => cell.phi = phase_rad,
            _ => {}
        }
        Ok(())
    }

    /// Apply a voltage on a global electrode using the linear V→rad model.
    pub fn set_electrode(&mut self, ep_idx_1based: usize, volts: f64) -> Result<(), EngineError> {
        self.set_phase(ep_idx_1based, volts * self.v_per_rad)
    }

    /// Borrow the `MziCell` at mesh column/row derived from instance index.
    /// We map instance `k` into the Clements grid by walking the columns in
    /// the same order ClementsArch::new fills them.
    fn cell_at(&mut self, k: usize) -> Option<&mut MziCell> {
        let n = self.n_ports;
        let mut count = 0usize;
        for col in 0..n {
            let row_start = col % 2;
            let mut row = row_start;
            while row + 1 < n {
                if count == k {
                    return self.arch.columns.get_mut(col)?.get_mut(row)?.as_mut();
                }
                count += 1;
                row += 2;
            }
        }
        None
    }
}

/// Collect top-cell labels with the given prefix, sorted by their numeric
/// suffix (so `op_l_1, op_l_2, op_l_3` come out in order).
fn sorted_labels_with_prefix(flat: &Flattened, prefix: &str) -> Vec<String> {
    let mut items: Vec<(usize, String)> = flat
        .labels
        .iter()
        .filter(|l| l.depth == 0 && l.string.starts_with(prefix))
        .map(|l| {
            let suffix = &l.string[prefix.len()..];
            let n: usize = suffix.parse().unwrap_or(usize::MAX);
            (n, l.string.clone())
        })
        .collect();
    items.sort_by_key(|(n, _)| *n);
    items.into_iter().map(|(_, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex64;

    fn engine_from_real_gds() -> Option<MziMatrixEngine> {
        let path = Path::new("src/sim/gds/Optical_Computing_MATRIX.py.gds");
        if !path.exists() {
            return None;
        }
        MziMatrixEngine::from_gds(path, "MATRIX_unit").ok()
    }

    #[test]
    fn loads_nine_mzis_and_36_electrodes() {
        let Some(eng) = engine_from_real_gds() else {
            return;
        };
        assert_eq!(eng.n_mzis(), 9, "expected 9 top-level MZI placements");
        assert_eq!(eng.n_electrodes(), 36, "9 MZI × 4 electrodes = 36");
        assert_eq!(eng.ports.inputs.len(), 3, "three left input ports");
        assert_eq!(eng.ports.outputs.len(), 5, "five right output ports");
    }

    #[test]
    fn identity_mesh_preserves_input() {
        let Some(eng) = engine_from_real_gds() else {
            return;
        };
        let n = eng.n_ports;
        let mut e = vec![Complex64::new(0.0, 0.0); n];
        e[0] = Complex64::new(1.0, 0.0);
        let y = eng.apply(&e).expect("apply");
        // Identity mesh: output == input (up to the diag_phases, which are 0).
        assert_eq!(y.len(), n);
        assert!((y[0].re - 1.0).abs() < 1e-9);
    }

    #[test]
    fn electrode_sets_theta_and_phi() {
        let Some(mut eng) = engine_from_real_gds() else {
            return;
        };
        eng.set_electrode(1, 1.0).unwrap(); // MZI[0] theta
        eng.set_electrode(2, 0.5).unwrap(); // MZI[0] phi
        // Round-trip: read back the unitary.
        let u = eng.to_unitary();
        assert_eq!(u.len(), eng.n_ports);
    }
}
