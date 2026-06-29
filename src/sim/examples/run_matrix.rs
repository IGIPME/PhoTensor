//! Run the MZI mesh engine on the real GDS file: load, identity apply,
//! then a small phase setting and a unitary program example.

use num_complex::Complex64;
use photensor_sim::mzi_array::MziMatrixEngine;
use std::path::Path;

fn main() {
    let path = Path::new("src/sim/gds/Optical_Computing_MATRIX.py.gds");
    let eng = MziMatrixEngine::from_gds(path, "MATRIX_unit").expect("build engine");

    println!("=== Engine built from GDS ===");
    println!("n_ports  = {}", eng.n_ports);
    println!("n_mzis   = {}", eng.n_mzis());
    println!("n_electrodes = {}", eng.n_electrodes());
    println!("inputs   = {:?}", eng.ports.inputs);
    println!("outputs  = {:?}", eng.ports.outputs);

    println!("\n=== Identity apply (input e0) ===");
    let mut e = vec![Complex64::new(0.0, 0.0); eng.n_ports];
    e[0] = Complex64::new(1.0, 0.0);
    let y = eng.apply(&e).expect("apply");
    for (i, v) in y.iter().enumerate() {
        println!("  out[{i}] = {:.4} {:+.4}i", v.re, v.im);
    }

    println!("\n=== Set electrode 1 (MZI0 θ) to π/2 and re-apply ===");
    let mut eng2 = eng;
    eng2.set_electrode(1, std::f64::consts::FRAC_PI_2 / eng2.v_per_rad)
        .unwrap();
    let y2 = eng2.apply(&e).expect("apply");
    for (i, v) in y2.iter().enumerate() {
        println!("  out[{i}] = {:.4} {:+.4}i", v.re, v.im);
    }

    println!("\n=== Program a Hadamard-like 5x5 (real orthogonal) target ===");
    let n = eng2.n_ports;
    // Build a simple real orthogonal target: identity with a 2x2 Hadamard
    // embedded in the top-left corner, so the decomposition stays well-defined.
    let h = 1.0 / 2f64.sqrt();
    let mut u = vec![vec![Complex64::new(0.0, 0.0); n]; n];
    for (i, row) in u.iter_mut().enumerate().take(n) {
        row[i] = Complex64::new(1.0, 0.0);
    }
    u[0][0] = Complex64::new(h, 0.0);
    u[0][1] = Complex64::new(h, 0.0);
    u[1][0] = Complex64::new(h, 0.0);
    u[1][1] = Complex64::new(-h, 0.0);
    eng2.program(&u);
    let realized = eng2.to_unitary();
    println!("realized unitary row 0:");
    for v in &realized[0] {
        print!("  {:.4}{:+.4}i", v.re, v.im);
    }
    println!();
}
