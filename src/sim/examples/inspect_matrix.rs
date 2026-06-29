//! Inspect the MZI mesh GDS: print top-level MZI placements and the port/ep
//! label map. Used to confirm the topology before writing the Clements engine.

use photensor_sim::gds::{Flattened, Layout};
use std::path::Path;

fn main() {
    let path = Path::new("src/sim/gds/Optical_Computing_MATRIX.py.gds");
    let layout = Layout::from_path(path, "MATRIX_unit").expect("load GDS");
    let flat = Flattened::from_layout(&layout);

    println!("top_cell: {}", flat.top_cell);
    println!(
        "db_unit_m: {:.3e}  user_unit_m: {:.3e}",
        flat.db_unit_m, flat.user_unit_m
    );
    println!(
        "total refs: {}  total labels: {}",
        flat.refs.len(),
        flat.labels.len()
    );

    println!("\n=== Depth-0 cell placements (top cell children) ===");
    let mut buckets: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for r in flat.refs.iter().filter(|r| r.depth == 0) {
        *buckets.entry(r.cell.clone()).or_default() += 1;
    }
    for (name, count) in &buckets {
        println!("  {name}: {count}");
    }

    println!("\n=== Top-level MZI placements (depth 0) ===");
    let mut mzis: Vec<_> = flat.top_level_instances("MZI").collect();
    mzis.sort_by_key(|r| (r.xform.tx, r.xform.ty));
    for (i, mzi) in mzis.iter().enumerate() {
        println!(
            "  MZI[{i}] origin=({}, {}) µm  angle={}°  mag={}  x_reflect={}",
            mzi.xform.tx as f64 * flat.db_unit_m * 1e6,
            mzi.xform.ty as f64 * flat.db_unit_m * 1e6,
            mzi.xform.angle_deg,
            mzi.xform.mag,
            mzi.xform.x_reflect,
        );
    }
    println!("n_top_level_mzis = {}", mzis.len());

    println!("\n=== Top-cell TEXT labels (depth 0) ===");
    let mut labels: Vec<_> = flat.labels.iter().filter(|l| l.depth == 0).collect();
    labels.sort_by_key(|l| l.string.clone());
    for l in &labels {
        println!(
            "  '{}' @ ({:.2}, {:.2}) µm  layer={}",
            l.string,
            l.x as f64 * flat.db_unit_m * 1e6,
            l.y as f64 * flat.db_unit_m * 1e6,
            l.layer,
        );
    }

    println!("\n=== Port counts ===");
    let op_l = flat.labels_with_prefix("op_l_").count();
    let op_r = flat.labels_with_prefix("op_r_").count();
    let ep = flat.labels_with_prefix("ep_").count();
    println!("op_l_ (inputs): {op_l}");
    println!("op_r_ (outputs): {op_r}");
    println!("ep_  (electrodes, all depths): {ep}");
}
