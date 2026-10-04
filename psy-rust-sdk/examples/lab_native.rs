//! Native counterpart of the wasm lab benchmarks: `cargo run --release --example lab_native --features lab-bench -- [threads]`.
fn main() -> anyhow::Result<()> {
    let threads: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
    if threads > 0 {
        rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;
    }
    use psy_rust_sdk::lab_bench::*;
    println!("rayon threads {}", rayon_threads());
    println!("load_local_circuits {:.1} ms", load_local_circuits()?);
    let z = zk_sign_inner(5)?;
    println!("zk_sign_inner {:?} ms", z.iter().map(|v| (v * 10.0).round() / 10.0).collect::<Vec<_>>());
    println!("deposit_inclusion_new {:.1} ms", deposit_inclusion_new()?);
    Ok(())
}
