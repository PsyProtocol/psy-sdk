use dashmap::DashMap;
use lazy_static::lazy_static;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::poseidon::PoseidonHash};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::{
    common::user_id::get_user_id_from_registration_id,
    hash::merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
};
use psy_data::qdata::{
    checkpoint::{PsyBlockState, PsyCheckpointLeaf},
    contract::{ContractCodeDefinition, PsyContractLeaf},
};
use serde::{Deserialize, Serialize};

const GLOBAL_USER_REGISTRATION_TREE_HEIGHT: u8 = 32;
const GLOBAL_CONTRACT_FUNCTION_TREE_HEIGHT: u8 = 16;

type F = GoldilocksField;
type Hash = QHashOut<F>;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisData {
    pub block_state: PsyBlockState,

    // checkpoint tree
    pub checkpoint_tree_proof: MerkleProofCore<Hash>,
    pub checkpoint_leaf_data: PsyCheckpointLeaf<F>,

    // user registration tree
    // pub user_registration_tree_store: MerkleProofCore<Hash>,
    pub user_registration_tree_root: Hash,

    // contract tree
    pub contract_tree_proof: MerkleProofCore<Hash>,
    pub contract0_code: ContractCodeDefinition,
    pub contract0_leaf_data: PsyContractLeaf<F>,
    // contract function tree
}

lazy_static! {
    pub static ref GENESIS_DATA: GenesisData =
        serde_json::from_str(include_str!("../../genesis_data.json")).expect("Failed to parse genesis_data.json (compile time embedded)");
    pub static ref CONTRACT0_FUNCTION_TREE_STORE: SimpleMerkleTree<PoseidonHash, QHashOut<F>> = {
        let mut contract_function_tree = SimpleMerkleTree::new(GLOBAL_CONTRACT_FUNCTION_TREE_HEIGHT);

        let contract_function_leaves = r#"[
            "dbb61de538a73b24046edd3f80cae580ef4e4b9a64bc2f477d20a8081f1f5f24",
            "00000000000000000000000000000000000000000000000100000000566e264c",
            "0224d6a570f77c2829777b78585fa1b4fd307aeecd5e2cae8229823d51cfc91d",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "346adab0bac01110e234647749e30b385551532f72d13b9b55101abe03be1a14",
            "00000000000000000000000000000000000000000000000100000000ae489a2f",
            "2b45c196fcdc2b195486f1ddf4d4639a36a18eb3cec6471986f3be68410efa06",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "b72e37f325b7a029f4d9cd779be0086b90853d1b6c29c11374e908077de4daef",
            "0000000000000000000000000000000000000000000000020000000015207137",
            "991700350f55fd982b23b3152c15294304f7fa1686a90d2e877d99ee5e926d89",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "0d5be52f0db6f4cae3e3e60d630affa459429c53eb762502187714c152505f02",
            "00000000000000000000000000000000000000000000000a00000000a04d1762",
            "59457b487f0728e1ce5f3d9077b721fefe14a4b7f882176bf2068704db42ed13",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "13a279216b6f227c6778af54933a209f79e6554695c5f010d3965fce145db283",
            "0000000000000000000000000000000000000000000000010000000088acd9ed",
            "26ee6cf91900805df4b145f70f7a555c531b6d6ceaa41b5f5cd5448e58b7268d",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "d0e0f8defc9d54da19575ae598f165015e6cba0a446886d136880903671ceaae",
            "000000000000000000000000000000000000000000000001000000001926d1ea",
            "5cf2524ce8e60fd18df5ca3d2e61c9191b1ea7cf69c7f6b4ec4511ccc3dde487",
            "0000000000000000000000000000000000000000000000000000000000000000"
        ]"#;
        let contract_function_leaves: Vec<QHashOut<F>> =
            serde_json::from_str(&contract_function_leaves).expect("Failed to parse contract function leaves (compile time embedded)");

        for (i, contract_function_leaf) in contract_function_leaves.iter().enumerate() {
            contract_function_tree.set_leaf(i as u64, *contract_function_leaf);
        }
        contract_function_tree
    };
    pub static ref USER_REGISTRATION_TREE_STORE: SimpleMerkleTree<PoseidonHash, QHashOut<F>> = {
        let mut user_registration_tree = SimpleMerkleTree::new(GLOBAL_USER_REGISTRATION_TREE_HEIGHT);

        let user_pks: Vec<QHashOut<F>> =
            serde_json::from_str(include_str!("../../genesis_users.json")).expect("Failed to parse genesis_users.json (compile time embedded)");

        for (i, user_pk) in user_pks.iter().enumerate() {
            user_registration_tree.set_leaf(i as u64, *user_pk);
        }
        user_registration_tree
    };
    pub static ref USER_ID_MAP: DashMap<Hash, u64> = {
        let user_id_map = DashMap::new();
        let user_pks: Vec<QHashOut<F>> =
            serde_json::from_str(include_str!("../../genesis_users.json")).expect("Failed to parse genesis_users.json (compile time embedded)");

        for (i, user_pk) in user_pks.iter().enumerate() {
            user_id_map.insert(*user_pk, get_user_id_from_registration_id(i as u64));
        }
        user_id_map
    };
}

mod tests {
    use super::*;

    #[test]
    fn test_user_registration_tree() -> anyhow::Result<()> {
        let user_registration_tree = &*USER_REGISTRATION_TREE_STORE;
        let user_1_proof = user_registration_tree.get_leaf(1);

        let expected_usere_1_proof = r#"{
            "root": "cbdbeaf8a4d0278c2ec0c1e1a7a52f62b9eb5f2730e0728d37d193e89b6172fa",
            "value": "366a3febbb0ae091bac26ef62871306cd8fbb98431c109218c5305e876a5c076",
            "index": 1,
            "siblings": [
                "f83aa03c3e21321421696202b90f4dab0a9f87237c231bbba58b8f93c799126e",
                "f1b78526b094031d3612a3509babb9312daac6e22787c7697f3b5ba022adc620",
                "ed7e09aac09cb44f749e85c631142b067cf1ac193278d2ba925679186481c436",
                "6b09b441445b0cdd8ea747ba3303c1b02f67330e21aad2aaef237879e7740510",
                "03e9e875f294f6dd80262d976f58299e7fe5ee66c2e111d786e42e8ebc288d82",
                "839e6a7391056a17b884ae0cd8edc0320b3a8d662d867fad558b84c12d25a3a4",
                "7304f60b07acdfb5ad63842616eb16bca167c79cea52d8c7ab883fed272c12b2",
                "a2460aaad0d911da4a582446eabdef14eccf0e7e80bacb7565b8ce4ae46ca37f",
                "c48682807a35c2ebd44ea418262db751016dba498452719394459b266e465ee0",
                "efb97df46ce2a5bbb27d9bafb0935dc71b5554aada20e6d59be73cd27c4d549d",
                "1ab3c77489f5880b33cade99beee590c7fdd0e07307522512a2bb7d4c8bbfdc5",
                "58e27e988fc281853419b91dc76ab181d3f428bd8bc13a372c7ec2186aef8a36",
                "be957df9b6c010c5ba70555c66bf93513d53f223fe3016bafb320d429835b388",
                "7cfd25a1d74ca068ecaa3d1e70ce83d9d92f5303a9d366cc3d62826cb5ebd580",
                "d294b69ed61b84c8be55b8b906e524fcfaf27f7cc502f7a21110f16ba01f6f63",
                "f2043cf127e8ed5148f735d03d685b343b4e5da584c96242c6d139e651275589",
                "cd1c1711f0119d7aa96aaf6a95845ba9b581801760066320020f3bb72c0e91fd",
                "ff54a6a67fddfd49df3b403afae8aee30aee12a36ec6c445460f806326cb9e4d",
                "de46231dce7c3a4a4f7d56b3fb3ae63e6158dc0ea58beea65844624c3350406a",
                "33a8e0b809ce2532ae94d561f2e16def904fa2e7b99bd3f1707d95a1148000a1",
                "7c9f51793bca6ffb713d0a918edaa60557184cbbc85f535743926baabe5db81f",
                "fa58391e7c0d394d317903270df6e518b34770c62a38e6697621f88cdcdfb5fd",
                "60e99b7ea5b1187d4293a24d51cc07ac39f874beb115877f8bd1878dd7f1026d",
                "c043477d124292017879345b4f881eb71d31cd8564acce2a617f3c6d0b4b8b44",
                "5793fc6d609c47c365b9470bc3e00cd4f19dece13278be693612ac9d812a8f8c",
                "e0c55886db8e5a00bfa58f8faf71ab1e1f12ae8ff82875c95b3c0f2c8ee070cc",
                "8f3c07c1b1e0b6c9c69aade405671398bf062e3f77dc0b13671c5e28b2f9dc9a",
                "06ff527899c10074411162bf4a7f70b84e6acab68322cba1e9e10aca93469e78",
                "08b8d7b96221d9f59ed49f4906c24becbe646c8d1b68665bf42d09eff74e4b90",
                "e0fd1bfa878b3cd2cc7e2bf5f351da7a2a1963d1913370406b4ae756e5e20763",
                "80faf1e491cd910ae2566bc52d26d7ea099b512bfeff20768a0dd4cf966a4a93",
                "20ca8d0d3b8c55d18b0f02df1c469ca317afad6c010c855f7765a145976afdbc"
            ]
        }"#;

        let expected_usere_1_proof: MerkleProofCore<Hash> = serde_json::from_str(expected_usere_1_proof)?;

        assert_eq!(user_1_proof, expected_usere_1_proof);

        let contract_function_proof = r#"{
            "root": "1999361a6b7af26d18a836b8bb03aff53a29f021897775bd32df01ed7e5b8296",
            "value": "dbb61de538a73b24046edd3f80cae580ef4e4b9a64bc2f477d20a8081f1f5f24",
            "index": 0,
            "siblings": [
                "00000000000000000000000000000000000000000000000100000000566e264c",
                "0c953b72f743263f157f5ad637495a6250e4a4cde9a6743f938205c5efd38fd2",
                "a8be332b4cb41fa2a53057ab78ad943484006b1857a6a5cbe7ac9bc75e8656f9",
                "f7f8bbc1aa6d150a71ce89cef272dd4c1770bd812e1a009ab78341dfe93b1044",
                "78b168fff07aebe2addd84aedd1b34739c808f9144d73aef853711f90287a37b",
                "d0053597686f6672b77e23f0fc59019786ac9b34bd97d439e9e6b5c8d15b61ae",
                "49561260080d30c3dda8f741c47dfb105a1d2a648eee8f0325225f1a5d49614a",
                "b768e4fc8b0b79f516c9da6ea83aa4b13c9a42c646c4c1f9e979ed3ee20855e3",
                "2bd367124a2989b3d31bd45195f9a9278d72cff3db0a7a5afe6fd7720cfd2916",
                "fcf1da35791ff4452cf0c633ee9d9197954ec02c35af849e3ca2442157c9f14e",
                "c27e8f4600af2a41707c71f51d338df791e919b1e4a3ea53ccf7b63f7b1140c3",
                "218bc75b3bc83675e1c5ac76b0d9d44c0d1baab6f05098e38d6ebaad0ab5d3c3",
                "61618c69e9d26f4c8ee39e4c215804e2fb01846fee718016ed2589168e839d21",
                "ec76a20799cf5dc50841b1fa4588f4f8c975d7aec7a1c669296ff821d8378f7f",
                "f55d5d12107b371efb4650fb6b8880811f7867621b8c1c1a0168a392cc7b542c",
                "6c9890682b94dee9cd45643c378df78c64e3f7a7160f8f0de73c5360c4b3ecd8"
            ]
        }"#;
        let expected_contract_function_proof: MerkleProofCore<Hash> = serde_json::from_str(&contract_function_proof)?;
        assert_eq!(CONTRACT0_FUNCTION_TREE_STORE.get_leaf(0), expected_contract_function_proof);

        Ok(())
    }
}
