use anyhow::Result;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use qed_core::job::id::{QProvingJobDataID, JobProof};
use serde_json::json;

use crate::subcommand::GetJobProofArgs;

pub async fn run(args: GetJobProofArgs) -> Result<()> {
    let job_id = parse_job_id(&args.job_id)?;

    println!("Fetching job proof for:");
    println!("  Checkpoint ID: {}", args.checkpoint_id);
    println!("  Job ID: {:?}", job_id);
    println!("  Coordinator URL: {}", args.coordinator_url);

    let proof = generate_batch_proof(&args.coordinator_url, args.checkpoint_id, job_id).await?;

    println!("\n=== Job Proof ===");
    println!("Job ID: {:?}", proof.job_id);
    println!("Value: {:?}", proof.value);
    println!("Root: {:?}", proof.root);
    println!("Number of siblings: {}", proof.siblings.len());

    for (idx, sibling) in proof.siblings.iter().enumerate() {
        println!("  Sibling[{}]:", idx);
        println!("    Hash: {:?}", sibling.hash);
        println!("    Is Left: {}", sibling.is_left);
        if let Some(parent_key) = &sibling.parent_public_key {
            println!("    Parent Public Key: {:?}", parent_key);
        }
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
) -> Result<JobProof> {
    let client = HttpClientBuilder::default()
        .build(coordinator_url)?;

    let job_ids = vec![job_id];
    let proofs: Vec<JobProof> = client
        .request("qed_generate_batch_proofs", rpc_params![checkpoint_id, job_ids])
        .await?;

    if proofs.is_empty() {
        anyhow::bail!("No proof returned for job ID {:?}", job_id);
    }

    Ok(proofs.into_iter().next().unwrap())
}
