use anyhow::Result;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use qed_core::job::id::{QProvingJobDataID, VariableHeightRewardMerkleProof};
use serde_json::json;

use crate::subcommand::GetJobProofArgs;

pub async fn run(args: GetJobProofArgs) -> Result<()> {
    let job_id = parse_job_id(&args.job_id)?;

    println!("Fetching job proof for:");
    println!("  Checkpoint ID: {}", args.checkpoint_id);
    println!("  Job ID: {:?}", job_id);
    println!("  Coordinator URL: {}", args.coordinator_url);

    let proof = generate_batch_proof(&args.coordinator_url, args.checkpoint_id, job_id).await?;

    println!("\n=== Variable Height Reward Merkle Proof ===");
    println!("Left Branch: {:?}", proof.left_branch);
    println!("Right Branch: {:?}", proof.right_branch);
    println!("Reward Leaf: {:?}", proof.reward_leaf);
    println!("Proof Height: {:?}", proof.proof_height);
    println!("Index: {:?}", proof.index);
    println!("Number of top siblings: {}", proof.top_siblings.len());

    for (idx, sibling) in proof.top_siblings.iter().enumerate() {
        println!("  Top Sibling[{}]:", idx);
        println!("    Branch: {:?}", sibling.sibling_branch);
        println!("    Reward Leaf: {:?}", sibling.sibling_reward_leaf);
    }

    println!("\n=== JSON Output ===");
    let json_output = serde_json::to_string_pretty(&proof)?;
    println!("{}", json_output);

    Ok(())
}

fn parse_job_id(hex_str: &str) -> Result<QProvingJobDataID> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    let bytes = hex::decode(hex_str)?;

    if bytes.len() != 24 {
        anyhow::bail!("Invalid job ID length: expected 24 bytes, got {}", bytes.len());
    }

    let job_id = QProvingJobDataID::try_from_byte_vec(&bytes)?;
    Ok(job_id)
}

async fn generate_batch_proof(
    coordinator_url: &str,
    checkpoint_id: u64,
    job_id: QProvingJobDataID,
) -> Result<VariableHeightRewardMerkleProof> {
    let client = HttpClientBuilder::default()
        .build(coordinator_url)?;

    let output_job_id = job_id.get_output_id();
    let job_ids = vec![output_job_id];
    let proofs: Vec<(VariableHeightRewardMerkleProof, QProvingJobDataID)> = client
        .request("generate_batch_variable_height_reward_proofs", rpc_params![checkpoint_id, job_ids])
        .await?;

    if proofs.is_empty() {
        anyhow::bail!("No proof returned for job ID {:?}", job_id);
    }

    Ok(proofs.into_iter().next().unwrap().0)
}
