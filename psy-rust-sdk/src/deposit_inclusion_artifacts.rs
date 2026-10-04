//! What the wallet needs of the DepositInclusion (shield deposit claim) circuit, without building
//! it: the minified circuit's fingerprint, common data and verifier data. They are fixed for a
//! given psy-node revision, so they are generated once on the host
//! (`cargo run --release --example gen_deposit_inclusion_artifacts`) and embedded;
//! `embedded_matches_circuit` checks the embedded bytes against a freshly built circuit.
//!
//! File layout: the fingerprint as 4 little-endian u64, then `VerifierCircuitData::to_bytes`
//! with `PsyGateSerializer`.

use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::circuit_data::{CommonCircuitData, VerifierCircuitData, VerifierOnlyCircuitData};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::serialization::PsyGateSerializer;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

pub struct DepositInclusionArtifacts {
    fingerprint: QHashOut<F>,
    data: VerifierCircuitData<F, C, D>,
}

impl DepositInclusionArtifacts {
    pub fn get_fingerprint(&self) -> QHashOut<F> {
        self.fingerprint
    }

    pub fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<F, D> {
        &self.data.common
    }

    pub fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        &self.data.verifier_only
    }

    /// From the embedded bytes.
    pub fn embedded() -> anyhow::Result<Self> {
        Self::from_bytes(include_bytes!("deposit_inclusion_artifacts.bin"))
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(bytes.len() > 32, "deposit inclusion artifacts too short");
        let word = |i: usize| u64::from_le_bytes(bytes[8 * i..8 * i + 8].try_into().expect("8 bytes"));
        let fingerprint = QHashOut::from_values(word(0), word(1), word(2), word(3));
        let data = VerifierCircuitData::<F, C, D>::from_bytes(bytes[32..].to_vec(), &PsyGateSerializer)
            .map_err(|e| anyhow::anyhow!("deposit inclusion verifier data: {e:?}"))?;
        Ok(Self { fingerprint, data })
    }

    /// The bytes to embed, from a built circuit (host only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn bytes_from_circuit(
        circuit: &psy_dpn_circuit::circuits::privacy::deposit_inclusion::DepositInclusionCircuit<C, D>,
    ) -> anyhow::Result<Vec<u8>> {
        use plonky2::field::types::PrimeField64;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        let mut out = Vec::new();
        for e in circuit.get_fingerprint().0.elements {
            out.extend_from_slice(&e.to_canonical_u64().to_le_bytes());
        }
        let data = VerifierCircuitData::<F, C, D> {
            verifier_only: circuit.get_verifier_config_ref().clone(),
            common: circuit.get_common_circuit_data_ref().clone(),
        };
        out.extend(data.to_bytes(&PsyGateSerializer).map_err(|e| anyhow::anyhow!("serialize: {e:?}"))?);
        Ok(out)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use psy_dpn_circuit::circuits::privacy::deposit_inclusion::DepositInclusionCircuit;

    /// The embedded artifacts must equal the circuit this psy-node revision builds; regenerate
    /// them with the example when this fails.
    #[test]
    fn embedded_matches_circuit() {
        let circuit = DepositInclusionCircuit::<C, D>::new();
        let built = DepositInclusionArtifacts::bytes_from_circuit(&circuit).unwrap();
        assert_eq!(built, include_bytes!("deposit_inclusion_artifacts.bin").to_vec(), "embedded artifacts are stale");
        let embedded = DepositInclusionArtifacts::embedded().unwrap();
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        assert_eq!(embedded.get_fingerprint(), circuit.get_fingerprint());
        assert_eq!(embedded.get_verifier_config_ref(), circuit.get_verifier_config_ref());
        assert_eq!(embedded.get_common_circuit_data_ref(), circuit.get_common_circuit_data_ref());
    }
}
