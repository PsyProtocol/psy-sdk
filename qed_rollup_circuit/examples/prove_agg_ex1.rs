use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_common_circuit::{circuits::traits::qstandard::QStandardCircuit, wallet::zk::{SimpleZKSignatureWallet, ZKSignatureBasicWalletProvider}};
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_rollup_circuit::circuits::cfc_placeholder::CFCPlaceholderCircuit;

//type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

fn run_prove_agg_example_sig() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("run_prove_agg_example_sig");
    timer.lap("start generate wallet circuit");
    let wallet = SimpleZKSignatureWallet::<C, D>::new();
    timer.lap("finish generate wallet circuit");

    let _sig_proof = wallet.zk_sign(
        QHashOut::from_values(13371, 13372, 13373, 13374),
        QHashOut::from_values(100, 200, 300, 400),
    ).unwrap();    
    timer.lap("generated signature proof");

    println!("zksig 1common data looks like:\n{:?}",wallet.wrapper_circuit.get_common_circuit_data_ref());

    Ok(())

}
fn run_prove_placeholder_example_1() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("run_prove_placeholder_example_1");

    timer.lap("start building circuit");
    let placeholder_cfc_circuit = CFCPlaceholderCircuit::<C, D>::new_with_minifier();
    timer.lap("finished building circuit");
    timer.lap("start proving");
    
    let _proof = placeholder_cfc_circuit.prove_seq_filler();
    timer.lap("finished proving");

    println!("placeholder 1common data looks like:\n{:?}",placeholder_cfc_circuit.get_common_circuit_data_ref());


    Ok(())
    
}


fn run_prove_agg_example_1() -> anyhow::Result<()> {
    let placeholder_cfc_circuit = CFCPlaceholderCircuit::<C, D>::new_with_minifier();


    let verifier_data_size = placeholder_cfc_circuit.get_verifier_config_ref().constants_sigmas_cap.0.len();
    println!("verifier_data_size: {}",verifier_data_size);
    //let agg_circuits = 






    Ok(())
    
}

fn main() {

    run_prove_placeholder_example_1().unwrap();
    run_prove_agg_example_sig().unwrap();
    run_prove_agg_example_1().unwrap();

}