//! Regenerates `src/deposit_inclusion_artifacts.bin` from the DepositInclusion circuit of the
//! psy-node revision in use: `cargo run --release --example gen_deposit_inclusion_artifacts`.
fn main() -> anyhow::Result<()> {
    use psy_dpn_circuit::circuits::privacy::deposit_inclusion::DepositInclusionCircuit;
    let circuit = DepositInclusionCircuit::<plonky2::plonk::config::PoseidonGoldilocksConfig, 2>::new();
    let bytes = psy_rust_sdk::deposit_inclusion_artifacts::DepositInclusionArtifacts::bytes_from_circuit(&circuit)?;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/deposit_inclusion_artifacts.bin");
    std::fs::write(path, &bytes)?;
    println!("wrote {} bytes to {path}", bytes.len());
    Ok(())
}
