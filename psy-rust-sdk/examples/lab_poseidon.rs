//! `cargo run --release --example lab_poseidon --features lab-bench -- [iterations]`: nanoseconds
//! per operation for the pieces of a Poseidon permutation on this machine.
fn main() -> anyhow::Result<()> {
    let iterations = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(200_000);
    println!("{}", psy_rust_sdk::lab_bench::poseidon_profile(iterations)?);
    Ok(())
}
