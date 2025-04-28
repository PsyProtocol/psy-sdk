use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use serde::{Deserialize, Serialize};

use super::args::RandomWalletArgs;
use anyhow::Result;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

#[derive(Serialize, Deserialize)]
struct RandomWalletOutputJSON {
    public_key: QHashOut<GoldilocksField>,
    private_key: QHashOut<GoldilocksField>,
}
pub fn run(_: RandomWalletArgs) -> Result<()> {
    let private_key = QHashOut::<GoldilocksField>::rand();
    println!("private key: {}", private_key);
    let mut debug_wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let zkey_info = debug_wallet.add_private_key_get_info(SimpleQEDPrivateKey {
        private_key: private_key,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&zkey_info)
            .map_err(|e| anyhow::format_err!("{}", e.to_string()))?
    );

    Ok(())
}
