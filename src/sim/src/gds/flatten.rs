//! Flatten the GDS cell hierarchy into world-coordinate placements.
//!
//! A `FlatRef` is one resolved placement of a referenced cell: the cell name,
//! a 2D affine transform (translation + angle + magnification + x-reflection)
//! and a depth counter (depth 0 == direct child of the top cell).
//! `FlatLabel` is a TEXT label resolved into world coordinates. The flattener
//! is intentionally geometry-only — it does not interpret device semantics.

use oxiphoton::geometry::gds::{GdsAref, GdsElement, GdsLibrary, GdsSref};

/// 2D affine transform used while flattening. Stored in DB units (i32) for
/// translation and float for angle (degrees) / magnification / x-reflection.
#[derive(Debug, Clone, Copy)]
pub struct Xform {
    pub tx: i64,
    pub ty: i64,
    pub angle_deg: f64,
    pub mag: f64,
    pub x_reflect: bool,
}

impl Xform {
    pub const IDENTITY: Xform = Xform {
        tx: 0,
        ty: 0,
        angle_deg: 0.0,
        mag: 1.0,
        x_reflect: false,
    };

    /// Compose `child` (the reference's local transform) onto `parent` (the
    /// accumulated transform of the containing cell).
    pub fn compose(self, child: &Xform) -> Xform {
        // Apply child's transform in the parent's coordinate frame:
        //   world = parent( child_local( point ) )
        // where child_local = R(angle)*S(mag)*M(x_reflect) + t.
        let rad = child.angle_deg.to_radians();
        let (s, c) = rad.sin_cos();
        let m = child.mag;
        let sx = if child.x_reflect { -1.0 } else { 1.0 };

        // Parent-transform a child displacement:
        //   parent_world( p + d ) = parent_world(p) + parent_linear(d)
        // but GDS semantics nest transforms as: world = parent_xform . child_xform
        let prad = self.angle_deg.to_radians();
        let (ps, pc) = prad.sin_cos();
        let pm = self.mag;
        let psx = if self.x_reflect { -1.0 } else { 1.0 };

        let combined_angle = self.angle_deg + child.angle_deg;
        let combined_mag = self.mag * child.mag;
        let combined_x_reflect = self.x_reflect ^ child.x_reflect;

        let dx = child.tx as f64;
        let dy = child.ty as f64;
        let wx = pm * (psx * pc * dx - psx * ps * dy);
        let wy = pm * (ps * dx + pc * dy);
        let _ = (s, c, m, sx);
        Xform {
            tx: self.tx + wx.round() as i64,
            ty: self.ty + wy.round() as i64,
            angle_deg: combined_angle,
            mag: combined_mag,
            x_reflect: combined_x_reflect,
        }
    }
}

/// One placement of one referenced cell, resolved to world coordinates.
#[derive(Debug, Clone)]
pub struct FlatRef {
    /// Name of the cell that was placed (e.g. `MZI`, `Mmi`, `wg`).
    pub cell: String,
    /// World-coordinate transform of this placement.
    pub xform: Xform,
    /// Depth in the hierarchy (0 == direct child of the top cell).
    pub depth: usize,
    /// Source-element name; helpful for debugging.
    pub kind: PlacementKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementKind {
    Sref,
    Aref {
        col: u16,
        row: u16,
        cols: u16,
        rows: u16,
    },
}

/// A TEXT label resolved into world coordinates (still in DB units).
#[derive(Debug, Clone)]
pub struct FlatLabel {
    pub string: String,
    /// World-coordinate origin (DB units).
    pub x: i64,
    pub y: i64,
    pub layer: u16,
    /// Depth where this label lives (0 == top cell).
    pub depth: usize,
}

/// Flattened view of a layout rooted at one top cell.
#[derive(Debug, Clone)]
pub struct Flattened {
    pub top_cell: String,
    pub db_unit_m: f64,
    pub user_unit_m: f64,
    pub refs: Vec<FlatRef>,
    pub labels: Vec<FlatLabel>,
}

impl Flattened {
    /// Recursively flatten the top cell of `layout`.
    pub fn from_layout(layout: &super::Layout) -> Self {
        let lib = layout.lib();
        let top = layout.top_cell();
        let mut refs = Vec::new();
        let mut labels = Vec::new();
        let mut visited: std::collections::HashSet<(String, i64, i64, u64, u64, bool)> =
            std::collections::HashSet::new();
        flatten_cell(
            lib,
            top,
            Xform::IDENTITY,
            0,
            &mut refs,
            &mut labels,
            &mut visited,
        );
        Flattened {
            top_cell: top.to_string(),
            db_unit_m: lib.db_unit_m,
            user_unit_m: lib.user_unit_m,
            refs,
            labels,
        }
    }

    /// All direct (depth==0) placements of `cell_name` inside the top cell.
    pub fn top_level_instances(&self, cell_name: &str) -> impl Iterator<Item = &FlatRef> {
        self.refs
            .iter()
            .filter(move |r| r.depth == 0 && r.cell == cell_name)
    }

    /// Labels whose string starts with `prefix`, in any cell of the tree.
    pub fn labels_with_prefix(&self, prefix: &str) -> impl Iterator<Item = &FlatLabel> {
        self.labels
            .iter()
            .filter(move |l| l.string.starts_with(prefix))
    }
}

fn flatten_cell(
    lib: &GdsLibrary,
    cell_name: &str,
    xform: Xform,
    depth: usize,
    refs: &mut Vec<FlatRef>,
    labels: &mut Vec<FlatLabel>,
    visited: &mut std::collections::HashSet<(String, i64, i64, u64, u64, bool)>,
) {
    // Guard against cyclic references. Depth cap also bounds work.
    if depth > 64 {
        return;
    }
    let key = (
        cell_name.to_string(),
        xform.tx,
        xform.ty,
        xform.angle_deg.to_bits(),
        xform.mag.to_bits(),
        xform.x_reflect,
    );
    if !visited.insert(key) {
        return;
    }
    let Some(cell) = lib.cells.iter().find(|c| c.name == cell_name) else {
        return;
    };

    for elem in &cell.elements {
        match elem {
            GdsElement::Sref(s) => {
                let child = xform_of_sref(s);
                let world = xform.compose(&child);
                refs.push(FlatRef {
                    cell: s.sname.clone(),
                    xform: world,
                    depth,
                    kind: PlacementKind::Sref,
                });
                flatten_cell(lib, &s.sname, world, depth + 1, refs, labels, visited);
            }
            GdsElement::Aref(a) => {
                let n = (a.cols as usize) * (a.rows as usize);
                for inst in aref_instances(a) {
                    let world = xform.compose(&inst.xform);
                    refs.push(FlatRef {
                        cell: a.ref_name.clone(),
                        xform: world,
                        depth,
                        kind: PlacementKind::Aref {
                            col: inst.col,
                            row: inst.row,
                            cols: a.cols,
                            rows: a.rows,
                        },
                    });
                    let _ = n; // n is informational only
                    flatten_cell(lib, &a.ref_name, world, depth + 1, refs, labels, visited);
                }
            }
            GdsElement::Text(t) => {
                // Resolve the label's origin through the current xform.
                let (wx, wy) = apply_xform(xform, t.origin.x as i64, t.origin.y as i64);
                labels.push(FlatLabel {
                    string: t.string.clone(),
                    x: wx,
                    y: wy,
                    layer: t.layer.layer,
                    depth,
                });
            }
            _ => {}
        }
    }
}

fn xform_of_sref(s: &GdsSref) -> Xform {
    Xform {
        tx: s.origin.x as i64,
        ty: s.origin.y as i64,
        angle_deg: s.angle_deg,
        mag: s.magnification,
        x_reflect: s.x_reflection,
    }
}

struct ArefInst {
    xform: Xform,
    col: u16,
    row: u16,
}

/// Expand an AREF into `cols * rows` individual placements.
/// GDS AREF semantics: `xy = [origin, col_anchor, row_anchor]`.
/// Each instance (i, j) sits at `origin + i*(col_anchor-origin) + j*(row_anchor-origin)`,
/// inheriting the AREF's angle/mag/reflection.
fn aref_instances(a: &GdsAref) -> Vec<ArefInst> {
    if a.xy.len() < 3 {
        return Vec::new();
    }
    let origin = a.xy[0];
    let col_anchor = a.xy[1];
    let row_anchor = a.xy[2];
    let dx = (
        (col_anchor.x - origin.x) as i64,
        (col_anchor.y - origin.y) as i64,
    );
    let dy = (
        (row_anchor.x - origin.x) as i64,
        (row_anchor.y - origin.y) as i64,
    );

    let mut out = Vec::with_capacity((a.cols as usize) * (a.rows as usize));
    for j in 0..a.rows {
        for i in 0..a.cols {
            let px = origin.x as i64 + (i as i64) * dx.0 + (j as i64) * dy.0;
            let py = origin.y as i64 + (i as i64) * dx.1 + (j as i64) * dy.1;
            out.push(ArefInst {
                xform: Xform {
                    tx: px,
                    ty: py,
                    angle_deg: a.angle_deg,
                    mag: a.magnification,
                    x_reflect: a.x_reflection,
                },
                col: i,
                row: j,
            });
        }
    }
    out
}

fn apply_xform(x: Xform, px: i64, py: i64) -> (i64, i64) {
    let rad = x.angle_deg.to_radians();
    let (s, c) = rad.sin_cos();
    let sx = if x.x_reflect { -1.0 } else { 1.0 };
    let fx = x.mag * (sx * c * px as f64 - sx * s * py as f64);
    let fy = x.mag * (s * px as f64 + c * py as f64);
    (x.tx + fx.round() as i64, x.ty + fy.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_compose_is_identity() {
        let x = Xform::IDENTITY.compose(&Xform::IDENTITY);
        assert_eq!(x.tx, 0);
        assert_eq!(x.ty, 0);
        assert!((x.angle_deg).abs() < 1e-9);
        assert!((x.mag - 1.0).abs() < 1e-9);
    }

    #[test]
    fn aref_expands_to_grid() {
        use oxiphoton::geometry::gds::{GdsAref, GdsPoint};
        let a = GdsAref::new(
            "sub",
            3,
            2,
            GdsPoint::new(0, 0),
            GdsPoint::new(100, 0),
            GdsPoint::new(0, 50),
        );
        let insts = aref_instances(&a);
        assert_eq!(insts.len(), 6);
        assert_eq!(insts[0].xform.tx, 0);
        assert_eq!(insts[5].xform.tx, 200);
        assert_eq!(insts[5].xform.ty, 50);
    }
}
